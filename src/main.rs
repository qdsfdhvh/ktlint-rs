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
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse_args();

    let mut config = KtlintConfig::load(&cli)?;
    if let Some(ref config_path) = cli.config {
        yaml_config::load_and_apply(&mut config, std::path::Path::new(config_path))?;
    }

    let files = FileCollector::new(&cli, &config).collect()?;
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
                    let file_config =
                        KtlintConfig::load_for_file(path).unwrap_or_else(|_| base_config.clone());
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

    let reporter = DiagnosticReporter::new(&cli);
    let exit_code = reporter.report(&violations);

    if cli.format && !violations.is_empty() {
        formatter::auto_fix(
            &files,
            &violations,
            config.indent_size,
            config.insert_final_newline,
        )?;
    }

    std::process::exit(exit_code);
}

