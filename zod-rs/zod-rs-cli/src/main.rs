//! # zod-rs-cli
//!
//! CLI tool for generating TypeScript Zod schemas from Rust source files.
//!
//! ## Usage
//!
//! ```bash
//! # Generate schemas from current directory
//! zod-rs generate
//!
//! # Generate schemas to a specific output directory
//! zod-rs generate --output ./generated
//!
//! # Watch mode for development
//! zod-rs generate --watch
//!
//! # Dry run to preview changes
//! zod-rs generate --dry-run
//!
//! # Initialize configuration
//! zod-rs init
//!
//! # Validate schemas are up-to-date
//! zod-rs validate --path ./generated/schemas.ts
//! ```

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use zod_rs_cli::{
    config::{CliArgs, Config, ConfigManager},
    error::{CliError, ParseError},
    generator::SchemaGenerator,
    parser::RustParser,
    scanner::SourceScanner,
    watcher::FileWatcher,
    writer::{FileWriter, WriteResult},
};

#[derive(Parser)]
#[command(name = "zod-rs")]
#[command(author, version, about = "Generate TypeScript Zod schemas from Rust types", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate TypeScript Zod schemas from Rust source files
    Generate {
        /// Input directory containing Rust source files
        #[arg(short, long, default_value = ".")]
        input: PathBuf,

        /// Output directory for generated TypeScript files
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Watch for file changes and regenerate
        #[arg(short, long)]
        watch: bool,

        /// Preview changes without writing files
        #[arg(long)]
        dry_run: bool,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Filter types by file path pattern (glob)
        #[arg(long)]
        filter: Option<String>,
    },

    /// Initialize a new zod-rs configuration file
    Init {
        /// Output path for configuration file
        #[arg(short, long, default_value = "zod-rs.toml")]
        output: PathBuf,

        /// Overwrite existing configuration file
        #[arg(long)]
        force: bool,
    },

    /// Validate that generated schemas are up-to-date
    Validate {
        /// Path to generated schemas file
        #[arg(short, long)]
        path: PathBuf,

        /// Input directory containing Rust source files
        #[arg(short, long, default_value = ".")]
        input: PathBuf,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            print_error(&e);
            match e {
                CliError::Validation(_) => ExitCode::from(2),
                _ => ExitCode::FAILURE,
            }
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Generate {
            input,
            output,
            watch,
            dry_run,
            config,
            filter,
        } => cmd_generate(input, output, watch, dry_run, config, filter),

        Commands::Init { output, force } => cmd_init(output, force),

        Commands::Validate {
            path,
            input,
            config,
        } => cmd_validate(path, input, config),
    }
}

/// Generate command implementation.
fn cmd_generate(
    input: PathBuf,
    output: Option<PathBuf>,
    watch: bool,
    dry_run: bool,
    config_path: Option<PathBuf>,
    filter: Option<String>,
) -> Result<(), CliError> {
    // Load configuration
    let config = ConfigManager::load(config_path.as_deref())?;
    let config = ConfigManager::merge_cli_args(
        config,
        &CliArgs {
            output: output.clone(),
            ..Default::default()
        },
    );

    if watch {
        run_watch_mode(&input, &config, filter.as_deref(), dry_run)
    } else {
        run_generate(&input, &config, filter.as_deref(), dry_run)
    }
}

/// Run schema generation once.
fn run_generate(
    input: &PathBuf,
    config: &Config,
    filter: Option<&str>,
    dry_run: bool,
) -> Result<(), CliError> {
    println!("{}", "Scanning for Rust source files...".cyan());

    // Scan for source files
    let mut scanner = SourceScanner::new(input);
    if let Some(pattern) = filter {
        scanner = scanner.with_filter(pattern)?;
    }

    let files = match scanner.scan_allow_empty() {
        Ok(files) => files,
        Err(e) => {
            println!("{} {}", "Warning:".yellow(), e);
            return Ok(());
        }
    };

    if files.is_empty() {
        println!("{}", "No Rust files found.".yellow());
        return Ok(());
    }

    println!("  Found {} Rust file(s)", files.len().to_string().green());

    // Parse source files
    println!("{}", "Parsing types with #[derive(ZodSchema)]...".cyan());

    let parser = RustParser::new().with_serde_compat(config.features.serde_compat);
    let (types, errors) = parser.parse_files(&files);

    // Report parse errors
    if !errors.is_empty() {
        println!("{} {} parse error(s):", "Warning:".yellow(), errors.len());
        for error in &errors {
            println!("  {}", format_parse_error(error));
        }
    }

    if types.is_empty() {
        println!("{}", "No types with #[derive(ZodSchema)] found.".yellow());
        return Ok(());
    }

    println!(
        "  Found {} type(s) with ZodSchema",
        types.len().to_string().green()
    );

    // Generate schemas
    println!("{}", "Generating Zod schemas...".cyan());

    let generator = SchemaGenerator::new(config.clone());
    let output = generator.generate(types)?;

    println!(
        "  Generated {} schema(s)",
        output.schemas.len().to_string().green()
    );

    // Write output
    let output_path = config.output.dir.join(&config.output.file);
    let writer = FileWriter::new(dry_run);

    match writer.write(&output_path, &output.content)? {
        WriteResult::Written { path, bytes } => {
            println!(
                "{} Written {} bytes to {}",
                "✓".green(),
                bytes,
                path.display()
            );
        }
        WriteResult::DryRun { content, path } => {
            println!(
                "{} Would write to {}:",
                "[dry-run]".yellow(),
                path.display()
            );
            println!("{}", "─".repeat(60).dimmed());
            println!("{}", content);
            println!("{}", "─".repeat(60).dimmed());
        }
    }

    Ok(())
}

/// Run in watch mode.
fn run_watch_mode(
    input: &PathBuf,
    config: &Config,
    filter: Option<&str>,
    dry_run: bool,
) -> Result<(), CliError> {
    println!("{}", "Starting watch mode...".cyan());
    println!("  Watching: {}", input.display());
    println!("  Press Ctrl+C to stop\n");

    // Initial generation
    run_generate(input, config, filter, dry_run)?;

    // Start watching
    let watcher = FileWatcher::new(input);
    let (_debouncer, rx) = watcher.watch()?;

    println!("\n{}", "Watching for changes...".cyan());

    while let Ok(event) = rx.recv() {
        if event.is_error() {
            println!(
                "{} {}",
                "Watch error:".red(),
                event.error_message().unwrap_or("Unknown error")
            );
            continue;
        }

        if let Some(path) = event.path() {
            println!("\n{} {}", "File changed:".cyan(), path.display());
        }

        // Regenerate
        if let Err(e) = run_generate(input, config, filter, dry_run) {
            println!("{} {}", "Generation error:".red(), e);
        }

        println!("\n{}", "Watching for changes...".cyan());
    }

    Ok(())
}

/// Init command implementation.
fn cmd_init(output: PathBuf, force: bool) -> Result<(), CliError> {
    if output.exists() && !force {
        println!(
            "{} Configuration file already exists: {}",
            "Error:".red(),
            output.display()
        );
        println!("  Use --force to overwrite");
        return Err(CliError::Validation(
            "Configuration file already exists".to_string(),
        ));
    }

    let content = ConfigManager::default_config_content();
    std::fs::write(&output, content)?;

    println!(
        "{} Created configuration file: {}",
        "✓".green(),
        output.display()
    );

    Ok(())
}

/// Validate command implementation.
fn cmd_validate(
    schema_path: PathBuf,
    input: PathBuf,
    config_path: Option<PathBuf>,
) -> Result<(), CliError> {
    println!("{}", "Validating schemas...".cyan());

    // Load existing schemas
    if !schema_path.exists() {
        return Err(CliError::Validation(format!(
            "Schema file not found: {}",
            schema_path.display()
        )));
    }

    let existing_content = std::fs::read_to_string(&schema_path)?;

    // Load configuration
    let config = ConfigManager::load(config_path.as_deref())?;

    // Scan and parse
    let scanner = SourceScanner::new(&input);
    let files = scanner.scan_allow_empty()?;

    let parser = RustParser::new().with_serde_compat(config.features.serde_compat);
    let (types, _) = parser.parse_files(&files);

    // Generate new schemas
    let generator = SchemaGenerator::new(config);
    let output = generator.generate(types)?;

    // Compare
    if existing_content.trim() == output.content.trim() {
        println!("{} Schemas are up-to-date", "✓".green());
        Ok(())
    } else {
        println!("{} Schemas are out of date", "✗".red());
        println!("  Run 'zod-rs generate' to update");
        Err(CliError::Validation("Schemas are out of date".to_string()))
    }
}

/// Print an error with formatting.
fn print_error(error: &CliError) {
    eprintln!("{} {}", "Error:".red().bold(), error);
}

/// Format a parse error for display.
fn format_parse_error(error: &ParseError) -> String {
    match error {
        ParseError::Syntax {
            file,
            line,
            column,
            message,
        } => {
            format!("{}:{}:{}: {}", file.display(), line, column, message)
        }
        ParseError::Attribute {
            file,
            line,
            message,
        } => {
            format!("{}:{}: {}", file.display(), line, message)
        }
        ParseError::UnsupportedType {
            file,
            line,
            type_name,
        } => {
            format!(
                "{}:{}: Unsupported type '{}'",
                file.display(),
                line,
                type_name
            )
        }
        ParseError::Io { file, source } => {
            format!("{}: {}", file.display(), source)
        }
        ParseError::Multiple(errors) => errors
            .iter()
            .map(format_parse_error)
            .collect::<Vec<_>>()
            .join("\n  "),
    }
}
