//! # pdf-cli
//!
//! CLI management tool for rsut-pdf-mcp knowledge bases.
//!
//! Provides terminal access to all management operations without
//! requiring a running server or AI client.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::all)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::await_holding_refcell_ref)]
#![deny(clippy::large_stack_frames)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::todo)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::unwrap_used)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pdf_core::management::{ConfigManager, HealthReporter};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "pdf-cli", version, about = "Manage rsut-pdf-mcp knowledge bases from the terminal")]
struct Cli {
    /// Path to the knowledge base directory
    #[arg(long, env = "KNOWLEDGE_BASE", default_value = ".")]
    knowledge_base: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show knowledge base health report
    Health,

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Trigger knowledge compilation
    Compile {
        /// Run incremental compile (only changed PDFs)
        #[arg(short, long)]
        incremental: bool,
    },

    /// Index management
    Index {
        /// Rebuild all indexes (fulltext + graph)
        #[arg(short, long)]
        rebuild: bool,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show all configuration values
    Show,

    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },

    /// Remove a configuration key
    Remove {
        /// Configuration key
        key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Health => cmd_health(&cli.knowledge_base),
        Commands::Config { action } => cmd_config(&cli.knowledge_base, action),
        Commands::Compile { incremental } => cmd_compile(&cli.knowledge_base, incremental).await,
        Commands::Index { rebuild } => cmd_index(&cli.knowledge_base, rebuild),
    }
}

fn cmd_health(kb_path: &Path) -> Result<()> {
    let reporter = HealthReporter::new(kb_path);
    let report = reporter
        .report()
        .context("Failed to generate health report")?;
    println!("{}", report);
    Ok(())
}

fn cmd_config(kb_path: &Path, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let mut cm = ConfigManager::new(kb_path);
            cm.load().context("Failed to load config")?;
            let data = cm.all();
            if data.is_empty() {
                println!("No configuration entries found.");
                return Ok(());
            }
            println!("Configuration ({} entries):", data.len());
            println!("{:<30} VALUE", "KEY");
            println!("{}", "─".repeat(60));
            let mut entries: Vec<_> = data.iter().collect();
            entries.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in entries {
                println!("{:<30} {}", k, v);
            }
            Ok(())
        }
        ConfigAction::Get { key } => {
            let mut cm = ConfigManager::new(kb_path);
            cm.load().context("Failed to load config")?;
            match cm.get(&key) {
                Some(value) => println!("{}", value),
                None => {
                    eprintln!("Key '{}' not found.", key);
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut cm = ConfigManager::new(kb_path);
            cm.load().context("Failed to load config")?;
            cm.set(&key, &value)
                .context("Failed to set config value")?;
            println!("Set '{}' = '{}'", key, value);
            Ok(())
        }
        ConfigAction::Remove { key } => {
            let mut cm = ConfigManager::new(kb_path);
            cm.load().context("Failed to load config")?;
            cm.remove(&key).context("Failed to remove config key")?;
            println!("Removed '{}'", key);
            Ok(())
        }
    }
}

async fn cmd_compile(kb_path: &Path, incremental: bool) -> Result<()> {
    if incremental {
        println!("Running incremental compile...");
        let config = pdf_core::ServerConfig::from_env().unwrap_or_default();
        let pipeline = std::sync::Arc::new(
            pdf_core::McpPdfPipeline::new(&config).context("Failed to create pipeline")?,
        );
        let engine = pdf_core::KnowledgeEngine::new(pipeline, kb_path)
            .context("Failed to create knowledge engine")?;
        let raw_dir = engine.raw_dir();
        let result = engine
            .incremental_compile(&raw_dir)
            .await
            .context("Incremental compile failed")?;
        println!(
            "Compile complete: {} compiled, {} skipped out of {} scanned",
            result.compiled, result.skipped, result.total_scanned
        );
    } else {
        println!("Full compile requires a PDF path. Use the MCP tool 'compile_to_wiki' for single-PDF compilation, or 'trigger_incremental_compile' for batch operations.");
        println!("To compile a specific PDF:");
        println!("  Use 'pdf-mcp' with the compile_to_wiki tool");
    }
    Ok(())
}

fn cmd_index(kb_path: &Path, rebuild: bool) -> Result<()> {
    if rebuild {
        let wiki_dir = kb_path.join("wiki");
        if !wiki_dir.exists() {
            eprintln!("Wiki directory not found at {}", wiki_dir.display());
            std::process::exit(1);
        }
        println!("Rebuilding indexes...");

        let ft_idx = pdf_core::FulltextIndex::open_or_create(kb_path)
            .context("Failed to open fulltext index")?;
        let ft_count = ft_idx
            .rebuild(&wiki_dir)
            .context("Failed to rebuild fulltext index")?;
        println!("Fulltext index: {} entries indexed", ft_count);

        let mut g_idx = pdf_core::GraphIndex::new();
        let g_count = g_idx
            .rebuild(&wiki_dir)
            .context("Failed to rebuild graph index")?;
        println!(
            "Graph index: {} nodes, {} edges",
            g_count,
            g_idx.edge_count()
        );

        println!("Index rebuild complete.");
    } else {
        println!("Use --rebuild to rebuild all indexes from wiki/ files.");
    }
    Ok(())
}
