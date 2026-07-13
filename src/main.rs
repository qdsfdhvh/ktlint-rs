mod baseline;
mod cli;
mod config;
mod discovery;
mod formatter;
mod git_hook;
mod parser;
mod reporter;
mod rules;
mod yaml_config;

#[cfg(test)]
mod format_tests;

use cli::Cli;
use config::KtlintConfig;
use discovery::FileCollector;
use parser::KotlinParser;
use rayon::prelude::*;
use reporter::DiagnosticReporter;
use rules::{RuleEngine, Violation};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse_args();

    // --- Git hook install/uninstall (no linting) ---
    if cli.install_git_hook {
        return git_hook::install_git_hook(&std::env::current_dir()?);
    }
    if cli.uninstall_git_hook {
        return git_hook::uninstall_git_hook(&std::env::current_dir()?);
    }

    let mut default_config = KtlintConfig::load(&cli)?;

    // Load YAML config if --config is provided
    if let Some(ref config_path) = cli.config {
        yaml_config::load_and_apply(&mut default_config, std::path::Path::new(config_path))?;
    }
    let files = FileCollector::new(&cli, &default_config).collect()?;

    let all_violations: Vec<Violation> = files
        .par_iter()
        .flat_map(|path| {
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let mut parser = KotlinParser::new();
            let tree = parser.parse(&source);
            let mut config =
                KtlintConfig::load_for_file(path).unwrap_or_else(|_| default_config.clone());
            // Inherit CLI-level settings into per-file config
            config.rule_set = default_config.rule_set;
            // Apply YAML config overrides
            if let Some(ref config_path) = cli.config {
                if let Err(e) =
                    yaml_config::load_and_apply(&mut config, std::path::Path::new(config_path))
                {
                    log::warn!("Failed to apply YAML config for {}: {}", path.display(), e);
                }
            }
            let engine = RuleEngine::new(&config);
            let violations = engine.check(&path.to_string_lossy(), &tree, &source);
            violations
        })
        .collect();

    // Generate baseline if --create-baseline (uses ALL violations, before filtering)
    if cli.create_baseline {
        let xml = baseline::Baseline::generate(&all_violations);
        let output_path = cli.baseline.as_deref().unwrap_or("baseline.xml");
        std::fs::write(output_path, &xml)?;
        eprintln!("Baseline written to: {}", output_path);
    }

    // Load baseline and filter (if --baseline is provided)
    let violations = if let Some(ref baseline_path) = cli.baseline {
        let baseline = baseline::Baseline::load(std::path::Path::new(baseline_path))?;
        baseline.filter(all_violations)
    } else {
        all_violations
    };

    let reporter = DiagnosticReporter::new(&cli);
    let exit_code = reporter.report(&violations);

    if cli.format && !violations.is_empty() {
        formatter::auto_fix(&files, &violations)?;
    }

    std::process::exit(exit_code);
}
