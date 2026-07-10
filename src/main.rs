mod cli;
mod config;
mod discovery;
mod formatter;
mod parser;
mod reporter;
mod rules;

use cli::Cli;
use config::KtlintConfig;
use discovery::FileCollector;
use parser::KotlinParser;
use reporter::DiagnosticReporter;
use rules::{RuleEngine, Violation};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse_args();

    // 1. Load configuration from .editorconfig
    let config = KtlintConfig::load(&cli)?;

    // 2. Discover Kotlin files
    let files = FileCollector::new(&cli, &config).collect()?;

    // 3. Process files
    let mut parser = KotlinParser::new();
    let engine = RuleEngine::new(&config);

    let mut all_violations: Vec<Violation> = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path)?;
        let tree = parser.parse(&source);
        let violations = engine.check(&path.to_string_lossy(), &tree, &source);
        all_violations.extend(violations);
    }

    // 4. Report
    let reporter = DiagnosticReporter::new(&cli);
    let exit_code = reporter.report(&all_violations);

    // 5. Auto-format if requested
    if cli.format && !all_violations.is_empty() {
        formatter::auto_fix(&files, &all_violations)?;
    }

    std::process::exit(exit_code);
}
