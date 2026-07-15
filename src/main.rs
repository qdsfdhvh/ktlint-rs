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
use reporter::DiagnosticReporter;
use rules::{RuleEngine, Violation};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse_args();

    if cli.install_git_hook {
        return git_hook::install_git_hook(&std::env::current_dir()?);
    }
    if cli.uninstall_git_hook {
        return git_hook::uninstall_git_hook(&std::env::current_dir()?);
    }

    let mut config = KtlintConfig::load(&cli)?;
    if let Some(ref config_path) = cli.config {
        yaml_config::load_and_apply(&mut config, std::path::Path::new(config_path))?;
    }

    let files = FileCollector::new(&cli, &config).collect()?;

    // Build RuleEngine ONCE (was per-file — O(n) to O(1))
    let engine = RuleEngine::new(&config);

    // Scoped thread pool with 4MB stacks — threads exit after scope
    let all_violations: Vec<Violation> = rayon::ThreadPoolBuilder::new()
        .stack_size(4 * 1024 * 1024)
        .build()?
        .install(|| {
            use rayon::prelude::*;
            files
                .par_iter()
                .flat_map(|path| {
                    let source = std::fs::read_to_string(path).unwrap_or_default();
                    let mut parser = KotlinParser::new();
                    let tree = parser.parse(&source);
                    engine.check(&path.to_string_lossy(), &tree, &source)
                })
                .collect::<Vec<_>>()
        });

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
        formatter::auto_fix(&files, &violations)?;
    }

    std::process::exit(exit_code);
}
