mod baseline;
mod cache;
mod cli;
mod config;
mod discovery;
mod formatter;
mod parser;
mod reporter;
mod resolver;
mod rules;
mod yaml_config;

#[cfg(test)]
mod format_tests;

use cli::Cli;
use config::KtlintConfig;
use discovery::FileCollector;
use parser::KotlinParser;
use reporter::DiagnosticReporter;
use rules::{RuleEngine, Violation};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn parity_path(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .trim_start_matches("./")
        .replace('\\', "/")
}

fn print_parity_files(files: &[PathBuf], root: &std::path::Path) -> anyhow::Result<()> {
    let mut paths: Vec<String> = files.iter().map(|path| parity_path(path, root)).collect();
    paths.sort();
    println!("{}", serde_json::to_string_pretty(&paths)?);
    Ok(())
}

fn print_parity_configs(files: &[PathBuf], base_config: &KtlintConfig) -> anyhow::Result<()> {
    let mut entries = Vec::new();
    for file in files {
        let config = KtlintConfig::load_for_file_with_base(file, base_config)
            .unwrap_or_else(|_| base_config.clone());
        let rules: BTreeMap<_, _> = config
            .rules
            .iter()
            .map(|(id, rule)| {
                let properties: BTreeMap<_, _> = rule.properties.iter().collect();
                (
                    id,
                    serde_json::json!({
                        "enabled": rule.enabled,
                        "properties": properties,
                    }),
                )
            })
            .collect();
        entries.push(serde_json::json!({
            "file": parity_path(file, &base_config.project_root),
            "code_style": match config.code_style {
                config::CodeStyle::KtlintOfficial => "ktlint_official",
                config::CodeStyle::AndroidStudio => "android_studio",
                config::CodeStyle::IntelliJIdea => "intellij_idea",
            },
            "indent_size": config.indent_size,
            "indent_style": match config.indent_style {
                config::IndentStyle::Space => "space",
                config::IndentStyle::Tab => "tab",
            },
            "tab_width": config.tab_width,
            "max_line_length": config.max_line_length,
            "insert_final_newline": config.insert_final_newline,
            "trim_trailing_whitespace": config.trim_trailing_whitespace,
            "rules": rules,
        }));
    }
    entries.sort_by(|left, right| left["file"].as_str().cmp(&right["file"].as_str()));
    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

fn print_rule_inventory(config: &KtlintConfig) -> anyhow::Result<()> {
    let inventory: Vec<_> = RuleEngine::inventory(config)
        .into_iter()
        .map(|rule| {
            serde_json::json!({
                "id": rule.id,
                "auto_fixable": rule.auto_fixable,
                "requires_type_resolution": rule.requires_type_resolution,
                "enabled_by_ruleset": rule.enabled_by_ruleset,
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&inventory)?);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse_args();

    let mut config = KtlintConfig::load(&cli)?;
    if let Some(ref config_path) = cli.config {
        yaml_config::load_and_apply(&mut config, std::path::Path::new(config_path))?;
    }
    if cli.print_rule_inventory {
        print_rule_inventory(&config)?;
        return Ok(());
    }

    let files = FileCollector::new(&cli, &config).collect()?;
    if cli.print_files {
        print_parity_files(&files, &config.project_root)?;
        return Ok(());
    }
    if cli.print_effective_config {
        print_parity_configs(&files, &config)?;
        return Ok(());
    }
    // Issue #53: load per-file editorconfig, build per-file engine
    let base_config = config.clone();
    // (engine removed — built per-file below)

    // Parallel lint with cache — collect results, then write cache sequentially
    let results: Vec<(PathBuf, Vec<Violation>)> = rayon::ThreadPoolBuilder::new()
        .stack_size(4 * 1024 * 1024)
        .build()?
        .install(|| {
            use rayon::prelude::*;
            files
                .par_iter()
                .map(|path| {
                    if let Some(cached) =
                        cache::get_cached(path, &base_config.project_root, &base_config)
                    {
                        return (path.clone(), cached);
                    }
                    // Load per-file .editorconfig
                    let file_config = KtlintConfig::load_for_file_with_base(path, &base_config)
                        .unwrap_or_else(|_| base_config.clone());
                    let engine = RuleEngine::new(&file_config);
                    let source = std::fs::read_to_string(path).unwrap_or_default();
                    let mut parser = KotlinParser::new();
                    let tree = parser.parse(&source);
                    let violations = engine.check(&path.to_string_lossy(), &tree, &source);
                    (path.clone(), violations)
                })
                .collect::<Vec<_>>()
        });

    // Save cache sequentially (no races)
    for (path, violations) in &results {
        cache::save_cached(path, violations, &config.project_root, &config);
    }

    // Collect all violations
    let all_violations: Vec<Violation> = results
        .iter()
        .flat_map(|(_, violations)| violations.clone())
        .collect();

    if cli.create_baseline {
        let xml = baseline::Baseline::generate(&all_violations);
        let output_path = cli.baseline.as_deref().unwrap_or("baseline.xml");
        std::fs::write(output_path, &xml)?;
        eprintln!("Baseline written to: {}", output_path);
    }

    let violations = if let Some(ref baseline_path) = cli.baseline {
        let baseline = baseline::Baseline::load(std::path::Path::new(baseline_path))?;
        baseline.filter(all_violations)
    } else {
        all_violations
    };

    // Blocker 5: warn on unmatched inputs
    if files.is_empty() && !cli.patterns.is_empty() {
        eprintln!("No files matched [{}]", cli.patterns.join(", "));
        if cli.strict {
            std::process::exit(1);
        }
    }

    let reporter = DiagnosticReporter::new(&cli);

    let exit_code = if cli.format && !violations.is_empty() {
        for file in &files {
            let file_config = KtlintConfig::load_for_file_with_base(file, &base_config)
                .unwrap_or_else(|_| base_config.clone());
            let file_name = file.to_string_lossy();
            let file_violations: Vec<Violation> = violations
                .iter()
                .filter(|violation| violation.file == file_name)
                .cloned()
                .collect();
            formatter::auto_fix(
                std::slice::from_ref(file),
                &file_violations,
                file_config.indent_size,
                file_config.insert_final_newline,
                &file_config.rules,
                file_config.code_style,
                file_config.max_line_length,
            )?;
        }
        // Re-lint with each file's effective EditorConfig, mirroring Spotless/ktlint.
        let mut post_violations = Vec::new();
        for file in &files {
            let file_config = KtlintConfig::load_for_file_with_base(file, &base_config)
                .unwrap_or_else(|_| base_config.clone());
            let engine = RuleEngine::new(&file_config);
            let source = std::fs::read_to_string(file).unwrap_or_default();
            let mut parser = KotlinParser::new();
            let tree = parser.parse(&source);
            post_violations.extend(engine.check(&file.to_string_lossy(), &tree, &source));
        }
        reporter.report(&post_violations)
    } else {
        reporter.report(&violations)
    };

    std::process::exit(exit_code);
}
