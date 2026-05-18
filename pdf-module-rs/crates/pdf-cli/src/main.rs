//! # rsut-pdf CLI
//!
//! Unified client for the rsut-pdf-mcp knowledge engine.
//!
//! ## Dual Mode Architecture
//!
//! - `--local` (default): Links directly to `pdf-core`, all operations local.
//! - `--remote`: Connects to a remote MCP server via HTTP API.
//!
//! ## File Transfer (Remote Mode)
//!
//! For cross-network scenarios, `compile --remote` automatically:
//! 1. Uploads the PDF via `POST /api/upload`
//! 2. Calls `compile_uploaded_pdf` MCP tool
//! 3. Returns compile results

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::all)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::await_holding_refcell_ref)]
#![deny(clippy::large_stack_frames)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::todo)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::unwrap_used)]

mod commands;
mod config;
mod local;
mod output;
mod proxy;
mod remote;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use commands::{CmdResult, Mode};
use output::OutputFormat;
use std::path::{Path, PathBuf};

// ── CLI Structure ──

#[derive(Parser)]
#[command(
    name = "rsut-pdf",
    version = "1.0.0",
    about = "Unified CLI for rsut-pdf-mcp knowledge engine — compile, search, manage"
)]
struct Cli {
    /// Use local mode: direct pdf-core integration, zero network
    /// Defaults to config file `mode` setting, or local if unset.
    #[arg(long, global = true, conflicts_with = "remote")]
    local: bool,

    /// Use remote mode: connect to a remote MCP server via HTTP
    #[arg(long, global = true, conflicts_with = "local")]
    remote: bool,

    /// Remote server URL (overrides config)
    #[arg(long, global = true, requires = "remote")]
    server: Option<String>,

    /// Auth token for remote server (overrides config/env)
    #[arg(long, global = true, requires = "remote")]
    token: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a PDF into the knowledge base
    Compile(commands::compile::CompileArgs),

    /// Micro-compile: extract text for current session, not saved to wiki
    #[command(name = "micro-compile")]
    MicroCompile(commands::compile::MicroCompileArgs),

    /// Recompile an existing wiki entry
    Recompile(commands::compile::RecompileArgs),

    /// Incremental compile of entire raw/ directory
    Incremental(commands::compile::IncrementalArgs),

    /// Full-text search across wiki entries
    Search(commands::query::SearchArgs),

    /// Get N-hop context for an entry (backlinks)
    Context(commands::query::ContextArgs),

    /// Export concept map as Mermaid.js
    #[command(name = "concept-map")]
    ConceptMap(commands::query::ConceptMapArgs),

    /// List orphan entries (no links)
    Orphans(commands::query::OrphansArgs),

    /// Knowledge base statistics
    Stats(commands::query::StatsArgs),

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Health check
    Health {
        /// Knowledge base path (local mode)
        #[arg(long)]
        knowledge_base: Option<String>,
    },

    /// Index management
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },

    /// Server management (Linux only, local)
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },

    /// Stdio proxy: forward stdio MCP to remote HTTP server
    Proxy,

    /// Workspace registry (multi knowledge base)
    Workspace {
        #[command(subcommand)]
        action: commands::platform::WorkspaceAction,
    },

    /// Git-like sync (local-first)
    Sync {
        #[command(subcommand)]
        action: commands::platform::SyncAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show all configuration values
    Show,
    /// Get a specific configuration value
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
    /// Reset configuration to defaults
    Reset,
}

#[derive(Subcommand)]
enum IndexAction {
    /// Rebuild all indexes (fulltext + graph)
    Rebuild {
        /// Knowledge base path
        #[arg(long)]
        knowledge_base: Option<String>,
    },
}

#[derive(Subcommand)]
enum ServerAction {
    /// Start the MCP server (Linux only)
    Start,
    /// Stop the MCP server
    Stop,
    /// Check server status
    Status,
}

// ── Main ──

#[tokio::main]
async fn main() -> Result<()> {
    // Register global panic hook for structured diagnostics
    std::panic::set_hook(Box::new(|info| {
        eprintln!("FATAL: {}", info);
    }));

    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();

    // Load config from ~/.rsut-pdf/config.toml
    let mut cfg = config::CliConfig::load()?;

    // CLI flags override config
    if let Some(ref server) = cli.server {
        cfg.server = Some(server.clone());
    }
    if let Some(ref token) = cli.token {
        cfg.token = Some(token.clone());
    }

    // Resolve mode from flags or config default
    let mode = if cli.remote {
        Mode::Remote
    } else if cli.local {
        Mode::Local
    } else {
        // Neither --local nor --remote explicitly set — fall back to config
        Mode::from_config(&cfg)
    };

    let format = if cli.json { OutputFormat::Json } else { OutputFormat::Text };

    let result = match &cli.command {
        Commands::Compile(args) => commands::compile::run_compile(&cfg, mode, args).await,
        Commands::MicroCompile(args) => {
            commands::compile::run_micro_compile(&cfg, mode, args).await
        }
        Commands::Recompile(args) => commands::compile::run_recompile(&cfg, mode, args).await,
        Commands::Incremental(args) => commands::compile::run_incremental(&cfg, mode, args).await,
        Commands::Search(args) => commands::query::run_search(&cfg, mode, args).await,
        Commands::Context(args) => commands::query::run_context(&cfg, mode, args).await,
        Commands::ConceptMap(args) => commands::query::run_concept_map(&cfg, mode, args).await,
        Commands::Orphans(args) => commands::query::run_orphans(&cfg, mode, args).await,
        Commands::Stats(args) => commands::query::run_stats(&cfg, mode, args).await,
        Commands::Config { action } => cmd_config(&mut cfg, action, format),
        Commands::Health { knowledge_base } => cmd_health(&cfg, mode, knowledge_base.as_deref()),
        Commands::Index { action } => cmd_index(&cfg, mode, action),
        Commands::Server { action } => cmd_server(action),
        Commands::Proxy => cmd_proxy(&cfg).await,
        Commands::Workspace { action } => commands::platform::run_workspace(&cfg, action, format)?,
        Commands::Sync { action } => commands::platform::run_sync(&cfg, mode, action, format)?,
    }?;

    result.print(format);
    Ok(())
}

// ── Command Handlers ──

/// Config management: operates on CLI config (~/.rsut-pdf/config.toml)
fn cmd_config(
    config: &mut config::CliConfig,
    action: &ConfigAction,
    _format: OutputFormat,
) -> Result<CmdResult> {
    match action {
        ConfigAction::Show => {
            let path = config::CliConfig::config_path()?;
            let content = if path.exists() {
                std::fs::read_to_string(&path)?
            } else {
                "No configuration file found (defaults apply).".to_string()
            };
            Ok(CmdResult::new(
                "CLI Configuration",
                serde_json::json!({
                    "config_path": path.to_string_lossy(),
                    "content": content,
                    "effective": {
                        "mode": config.mode,
                        "server": config.server,
                        "token": config.token.as_ref().map(|_| "***"),
                        "knowledge_base": config.knowledge_base,
                        "remote_knowledge_base": config.remote_knowledge_base,
                    }
                }),
            ))
        }
        ConfigAction::Get { key } => {
            let value = match key.as_str() {
                "mode" => config.mode.clone(),
                "server" => config.server.clone().unwrap_or_default(),
                "token" => config.token.clone().unwrap_or_default(),
                "knowledge_base" | "kb" => config
                    .knowledge_base
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "remote_knowledge_base" | "remote_kb" => {
                    config.remote_knowledge_base.clone().unwrap_or_default()
                }
                _ => anyhow::bail!("Unknown config key: {}", key),
            };
            Ok(CmdResult::new(format!("Config: {}", key), serde_json::json!({ key: value })))
        }
        ConfigAction::Set { key, value } => {
            match key.as_str() {
                "mode" => {
                    // Accept both "local"/"remote" and "--local"/"--remote"
                    let mode_str = match value.as_str() {
                        "--local" | "local" => "local",
                        "--remote" | "remote" => "remote",
                        _ => anyhow::bail!("Invalid mode: {}. Use 'local' or 'remote'.", value),
                    };
                    config.mode = mode_str.to_string();
                }
                "server" => config.server = Some(value.clone()),
                "token" => config.token = Some(value.clone()),
                "knowledge_base" | "kb" => {
                    config.knowledge_base = Some(PathBuf::from(value));
                }
                "remote_knowledge_base" | "remote_kb" => {
                    config.remote_knowledge_base = Some(value.clone());
                }
                _ => anyhow::bail!("Unknown config key: {}", key),
            }
            config.save()?;
            Ok(CmdResult::new(
                "Config Set",
                serde_json::json!({ "key": key, "value": value, "status": "ok" }),
            ))
        }
        ConfigAction::Reset => {
            *config = config::CliConfig::default();
            config.save()?;
            Ok(CmdResult::new(
                "Config Reset",
                serde_json::json!({ "status": "ok", "message": "Configuration reset to defaults." }),
            ))
        }
    }
}

/// Health check
fn cmd_health(config: &config::CliConfig, mode: Mode, kb_path: Option<&str>) -> Result<CmdResult> {
    match mode {
        Mode::Local => {
            let kb = commands::resolve_kb_path(config, kb_path.map(PathBuf::from).as_deref());
            let result = local::health(&kb)?;
            Ok(CmdResult::new("Health Report", result))
        }
        Mode::Remote => {
            let _server = config.server.as_deref().ok_or_else(|| {
                anyhow::anyhow!("No remote server configured. Use --server or config set.")
            })?;
            let client = commands::build_remote_client(config)?;
            let result = futures::executor::block_on(client.health())?;
            Ok(CmdResult::new("Server Health", result))
        }
    }
}

/// Index management
fn cmd_index(config: &config::CliConfig, mode: Mode, action: &IndexAction) -> Result<CmdResult> {
    match action {
        IndexAction::Rebuild { knowledge_base } => match mode {
            Mode::Local => {
                let kb = commands::resolve_kb_path(
                    config,
                    knowledge_base.as_ref().map(PathBuf::from).as_deref(),
                );
                let result = local::rebuild_index(&kb)?;
                Ok(CmdResult::new("Index Rebuild", result))
            }
            Mode::Remote => {
                let client = commands::build_remote_client(config)?;
                let result = futures::executor::block_on(async {
                    client
                        .call_tool(
                            "rebuild_index",
                            serde_json::json!({
                                "knowledge_base": commands::resolve_remote_kb(
                                    config,
                                    knowledge_base.as_deref(),
                                ),
                            }),
                        )
                        .await
                })?;
                Ok(CmdResult::new("Index Rebuild", result))
            }
        },
    }
}

/// Server management (local Linux only)
fn cmd_server(action: &ServerAction) -> Result<CmdResult> {
    match action {
        ServerAction::Start => {
            let bin_path =
                std::env::current_exe().context("Cannot determine current executable path")?;
            let bin_dir = bin_path.parent().unwrap_or(&bin_path);

            let mcp_bin = find_server_binary(bin_dir);

            match mcp_bin {
                Some(path) => {
                    let child = std::process::Command::new(&path)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::inherit())
                        .spawn()
                        .context(format!("Failed to start server: {}", path.display()))?;

                    let pid = child.id();
                    // Detach — don't wait for child
                    std::mem::forget(child);

                    Ok(CmdResult::new(
                        "Server Start",
                        serde_json::json!({
                            "status": "started",
                            "pid": pid,
                            "binary": path.to_string_lossy(),
                            "message": format!("Server started (PID: {})", pid),
                        }),
                    ))
                }
                None => {
                    anyhow::bail!(
                        "Cannot find 'pdf-mcp' binary. Make sure it's installed and in PATH."
                    );
                }
            }
        }
        ServerAction::Stop => {
            let output = std::process::Command::new("pkill")
                .arg("-f")
                .arg("pdf-mcp")
                .output()
                .context("Failed to run pkill")?;

            if output.status.success() {
                Ok(CmdResult::new("Server Stop", serde_json::json!({ "status": "stopped" })))
            } else {
                Ok(CmdResult::new(
                    "Server Stop",
                    serde_json::json!({
                        "status": "not_running",
                        "message": "No running pdf-mcp process found.",
                    }),
                ))
            }
        }
        ServerAction::Status => {
            let output = std::process::Command::new("pgrep")
                .arg("-f")
                .arg("pdf-mcp")
                .output()
                .context("Failed to run pgrep")?;

            if output.status.success() {
                let pids = String::from_utf8_lossy(&output.stdout);
                let pid_list: Vec<&str> = pids.lines().collect();
                Ok(CmdResult::new(
                    "Server Status",
                    serde_json::json!({
                        "running": true,
                        "pids": pid_list,
                        "count": pid_list.len(),
                    }),
                ))
            } else {
                Ok(CmdResult::new(
                    "Server Status",
                    serde_json::json!({ "running": false, "pids": [], "count": 0 }),
                ))
            }
        }
    }
}

/// Stdio proxy mode
async fn cmd_proxy(config: &config::CliConfig) -> Result<CmdResult> {
    proxy::run_proxy(config).await?;
    Ok(CmdResult::new("Proxy", serde_json::json!({"status": "shutdown"})))
}

/// Find the pdf-mcp server binary
fn find_server_binary(search_dir: &Path) -> Option<PathBuf> {
    // Check same directory as CLI
    let candidate = search_dir.join("pdf-mcp");
    if candidate.exists() {
        return Some(candidate);
    }

    // Check PATH
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join("pdf-mcp");
            if candidate.exists() { Some(candidate) } else { None }
        })
    })
}
