mod cli;
mod config;
mod discovery;
mod formatter;
mod parser;
mod reporter;
mod rules;

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
    let default_config = KtlintConfig::load(&cli)?;
    let files = FileCollector::new(&cli, &default_config).collect()?;

    let use_compat = cli.compat || std::env::var("KTLINT_COMPAT").is_ok();

    let all_violations: Vec<Violation> = files
        .par_iter()
        .flat_map(|path| {
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let mut parser = KotlinParser::new();
            let tree = parser.parse(&source);
            let config =
                KtlintConfig::load_for_file(path).unwrap_or_else(|_| default_config.clone());
            let violations = if use_compat {
                rules::compat::KtlintCompatEngine::new(&config).check(
                    &path.to_string_lossy(),
                    &tree,
                    &source,
                )
            } else {
                let engine = RuleEngine::new(&config);
                engine.check(&path.to_string_lossy(), &tree, &source)
            };
            rules::suppress::filter_suppressed(violations, &source)
        })
        .collect();

    let reporter = DiagnosticReporter::new(&cli);
    let exit_code = reporter.report(&all_violations);

    if cli.format && !all_violations.is_empty() {
        formatter::auto_fix(&files, &all_violations)?;
    }

    std::process::exit(exit_code);
}
