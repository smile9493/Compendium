// DEVIATION: This is an installer CLI tool (rapid/prototype mode).
// Unwrap on user I/O is acceptable since stdin failure is terminal.
// Lints are warn-level rather than deny to keep the tool accessible
// for rapid iteration without blocking the build on user-experience code.
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(clippy::all)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

const DEFAULT_INSTALL_DIR: &str = "/opt/pdf-module";
const DEFAULT_VLM_ENDPOINT: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const DEFAULT_VLM_MODEL: &str = "glm-4v-flash";
const DEFAULT_SERVER_URL: &str = "http://localhost:8000";

// ─────────────────────────── CLI Parser ───────────────────────────

#[derive(Parser)]
#[command(name = "pdf-mcp-cli", version = "0.1.4")]
#[command(author = "PDF Module Team")]
#[command(about = "PDF Module MCP 配置管理工具", long_about = None)]
struct Cli {
    /// MCP Server URL for file operations
    #[arg(global = true, long, default_value = DEFAULT_SERVER_URL)]
    server: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 初始化配置和环境
    Init,

    /// 配置 GLM API Key 和参数
    Config {
        #[arg(short, long)]
        key: Option<String>,

        #[arg(short, long)]
        model: Option<String>,

        #[arg(short, long)]
        endpoint: Option<String>,
    },

    /// 查看配置状态和服务状态
    Status,

    /// 生成 MCP 客户端配置
    GenerateConfig {
        #[arg(short, long)]
        output: Option<String>,
    },

    /// 显示配置说明和使用指南
    Info {
        /// 配置说明部分: server | client | security
        #[arg(short, long)]
        section: Option<String>,
    },

    /// 管理系统服务 (start/stop/restart/status)
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },

    /// 启动 Web Dashboard (别名 dashboard)
    Dashboard {
        /// Dashboard 端口
        #[arg(short, long, default_value = "8000")]
        port: u16,
    },

    /// 进入交互式菜单模式
    Interactive,

    /// 上传 PDF 文件到服务器
    Upload {
        /// PDF 文件路径
        file: String,
    },

    /// 列出服务器上的 PDF 文件
    List,

    /// 测试 PDF 文件处理
    Test {
        /// PDF 文件路径
        #[arg(short, long)]
        file: Option<String>,
    },

    // ── Legacy commands (kept for backward compat) ──

    /// [Legacy] 启动服务 — 推荐使用 `service start`
    Start {
        #[arg(short, long)]
        web: bool,
    },

    /// [Legacy] 停止服务 — 推荐使用 `service stop`
    Stop,

    /// [Legacy] 重启服务 — 推荐使用 `service restart`
    Restart,

    /// 查看日志
    Logs {
        #[arg(short, long, default_value = "20")]
        lines: u16,

        #[arg(short, long)]
        follow: bool,
    },

    /// 查看进程列表
    Ps,
}

#[derive(Subcommand)]
enum ServiceAction {
    /// 启动服务
    Start,
    /// 停止服务
    Stop,
    /// 重启服务
    Restart,
    /// 查看服务状态
    Status,
}

// ─────────────────────────── Config Types ───────────────────────────

#[derive(Serialize, Deserialize, Debug)]
struct EnvConfig {
    vlm_api_key: String,
    vlm_model: String,
    vlm_endpoint: String,
    dashboard_port: u16,
    rust_log: String,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            vlm_api_key: String::new(),
            vlm_model: DEFAULT_VLM_MODEL.to_string(),
            vlm_endpoint: DEFAULT_VLM_ENDPOINT.to_string(),
            dashboard_port: 8000,
            rust_log: "info".to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct PdfFileInfo {
    name: String,
    path: String,
    size: u64,
    #[serde(default)]
    pages: Option<u32>,
    #[serde(default)]
    created: Option<String>,
}

// ─────────────────────────── McpManager ───────────────────────────

struct McpManager {
    install_dir: String,
    env_file: String,
    pid_file: String,
    server_url: String,
}

impl McpManager {
    fn new(install_dir: Option<String>, server_url: Option<String>) -> Self {
        let dir = install_dir.unwrap_or_else(|| DEFAULT_INSTALL_DIR.to_string());
        Self {
            install_dir: dir.clone(),
            env_file: format!("{}/.env.local", dir),
            pid_file: format!("{}/.service.pid", dir),
            server_url: server_url.unwrap_or_else(|| DEFAULT_SERVER_URL.to_string()),
        }
    }

    // ── Config load / save ──

    fn load_config(&self) -> EnvConfig {
        if !Path::new(&self.env_file).exists() {
            return EnvConfig::default();
        }

        let content = fs::read_to_string(&self.env_file).unwrap_or_default();
        let mut config = EnvConfig::default();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "VLM_API_KEY" => config.vlm_api_key = value.trim().to_string(),
                    "VLM_MODEL" => config.vlm_model = value.trim().to_string(),
                    "VLM_ENDPOINT" => config.vlm_endpoint = value.trim().to_string(),
                    "DASHBOARD_PORT" => {
                        config.dashboard_port = value.trim().parse().unwrap_or(8000)
                    }
                    "RUST_LOG" => config.rust_log = value.trim().to_string(),
                    _ => {}
                }
            }
        }

        config
    }

    fn save_config(&self, config: &EnvConfig) -> std::io::Result<()> {
        let content = format!(
            r#"# PDF Module MCP 配置

VLM_API_KEY={}
VLM_MODEL={}
VLM_ENDPOINT={}

DASHBOARD_PORT={}
STORAGE_TYPE=local
STORAGE_LOCAL_DIR={}/data

RUST_LOG={}
"#,
            config.vlm_api_key,
            config.vlm_model,
            config.vlm_endpoint,
            config.dashboard_port,
            self.install_dir,
            self.install_dir,
            config.rust_log,
        );
        fs::write(&self.env_file, content)
    }

    // ── Banner ──

    fn show_banner(&self) {
        println!(
            "\n{}",
            "██████╗  ██████╗ ██╗     ██╗     ██╗███╗   ██╗ ██████╗ "
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "██╔══██╗██╔═══██╗██║     ██║     ██║████╗  ██║██╔════╝ "
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "██████╔╝██║   ██║██║     ██║     ██║██╔██╗ ██║██║  ███╗"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "██╔═══╝ ██║   ██║██║     ██║     ██║██║╚██╗██║██║   ██║"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "██║     ╚██████╔╝███████╗███████╗██║██║ ╚████║╚██████╔╝"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "╚═╝      ╚═════╝ ╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝ "
                .cyan()
                .bold()
        );
        println!("\n{}", "PDF Module MCP CLI v0.1.4".green().bold());
        println!("{}", "配置管理工具".blue());
        println!();
    }

    // ── Process helpers ──

    fn check_process(&self, name: &str) -> Option<u32> {
        let output = Command::new("pgrep").args(["-f", name]).output().ok()?;
        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout);
            pid_str.lines().next()?.trim().parse().ok()
        } else {
            None
        }
    }

    fn check_mcp_server(&self) -> Option<u32> {
        let output = Command::new("ps").args(["aux"]).output().ok()?;
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("/opt/pdf-module/pdf-mcp")
                && !line.contains("pdf-dashboard")
                && !line.contains("--version")
                && !line.contains("--help")
                && !line.contains("dashboard")
            {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        return Some(pid);
                    }
                }
            }
        }
        None
    }

    fn check_dashboard(&self) -> Option<u32> {
        self.check_process("/opt/pdf-module/pdf-dashboard")
    }

    fn check_port(&self, port: u16) -> bool {
        Command::new("ss")
            .args(["-tlnp"])
            .output()
            .map(|output| {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.contains(&format!(":{}", port))
            })
            .unwrap_or(false)
    }

    fn save_pid(&self, pid: u32) {
        let _ = fs::write(&self.pid_file, pid.to_string());
    }

    // ── Commands ──

    fn cmd_init(&self) {
        println!("\n{}", ">>> 初始化配置".cyan().bold());

        print!("  {} 创建目录结构...", "→".blue());
        fs::create_dir_all(&self.install_dir).ok();
        fs::create_dir_all(format!("{}/logs", self.install_dir)).ok();
        fs::create_dir_all(format!("{}/data", self.install_dir)).ok();
        fs::create_dir_all(format!("{}/wiki/raw", self.install_dir)).ok();
        fs::create_dir_all(format!("{}/wiki/wiki", self.install_dir)).ok();
        fs::create_dir_all(format!("{}/wiki/scheme", self.install_dir)).ok();
        println!(" {}", "✓".green());

        print!("  {} 创建配置文件...", "→".blue());
        if !Path::new(&self.env_file).exists() {
            let config = EnvConfig::default();
            self.save_config(&config).ok();
            println!(" {}", "✓".green());
        } else {
            println!(" {}", "已存在".blue());
        }

        println!("\n{} 初始化完成！", "✓".green());
        println!("  安装目录: {}", self.install_dir);
    }

    fn cmd_config(&self, key: Option<String>, model: Option<String>, endpoint: Option<String>) {
        println!("\n{}", ">>> 配置 API".cyan().bold());

        let mut config = self.load_config();
        let mut changed = false;

        if let Some(k) = key {
            config.vlm_api_key = k;
            changed = true;
            println!("  {} API Key 已设置", "✓".green());
        } else if config.vlm_api_key.is_empty() {
            println!("\n  获取 API Key: https://open.bigmodel.cn/ -> 控制台 -> API Keys\n");
            let api_key: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("请输入 API Key")
                .interact_text()
                .expect("failed to read API key from stdin");
            if !api_key.is_empty() {
                config.vlm_api_key = api_key;
                changed = true;
            }
        }

        if let Some(m) = model {
            config.vlm_model = m;
            changed = true;
            println!("  {} 模型: {}", "✓".green(), config.vlm_model);
        }

        if let Some(e) = endpoint {
            config.vlm_endpoint = e;
            changed = true;
            println!("  {} 端点已设置", "✓".green());
        }

        if changed {
            self.save_config(&config).ok();
            println!("\n{} 配置已保存", "✓".green());
        }

        self.show_config_summary(&config);
    }

    fn cmd_status(&self) {
        println!("\n{}", ">>> 服务状态".cyan().bold());

        let config = self.load_config();

        println!("\n  {}", "配置:".yellow());
        if config.vlm_api_key.is_empty() {
            println!("    {} API Key: 未配置", "✗".red());
        } else {
            let masked = format!(
                "{}****",
                &config.vlm_api_key[..8.min(config.vlm_api_key.len())]
            );
            println!("    {} API Key: {}", "✓".green(), masked);
        }
        println!("    {} 模型: {}", "→".blue(), config.vlm_model);
        println!("    {} 端口: {}", "→".blue(), config.dashboard_port);

        println!("\n  {}", "进程:".yellow());

        if let Some(pid) = self.check_mcp_server() {
            println!("    {} MCP Server 运行中 (PID: {})", "✓".green(), pid);
        } else {
            println!("    {} MCP Server 未运行 (按需启动)", "○".blue());
        }

        if let Some(pid) = self.check_dashboard() {
            println!("    {} Dashboard API 运行中 (PID: {})", "✓".green(), pid);
            println!("      访问: http://localhost:{}", config.dashboard_port);
        } else {
            println!("    {} Dashboard API 未运行", "○".blue());
        }
    }

    fn cmd_ps(&self) {
        println!("\n{}", ">>> 进程列表".cyan().bold());

        println!(
            "\n  {:<8} {:<20} {}",
            "PID".cyan(),
            "名称".cyan(),
            "状态".cyan()
        );
        println!("  {}", "-".repeat(40));

        let processes = vec![
            ("MCP Server", self.check_mcp_server()),
            ("Dashboard API", self.check_dashboard()),
        ];

        let mut found = false;
        for (name, pid_opt) in processes {
            if let Some(pid) = pid_opt {
                println!(
                    "  {:<8} {:<20} {}",
                    pid.to_string().white(),
                    name,
                    "运行中".green()
                );
                found = true;
            }
        }

        if !found {
            println!("  {}", "无运行中的进程".blue());
        }
    }

    fn cmd_generate_config(&self, output: Option<String>) {
        println!("\n{}", ">>> 生成客户端配置".cyan().bold());

        let mcp_config = serde_json::json!({
            "mcpServers": {
                "pdf-module": {
                    "command": format!("{}/pdf-mcp", self.install_dir),
                    "env": {
                        "VLM_API_KEY": "",
                        "VLM_MODEL": DEFAULT_VLM_MODEL,
                        "VLM_ENDPOINT": DEFAULT_VLM_ENDPOINT
                    }
                }
            }
        });

        let config_str = serde_json::to_string_pretty(&mcp_config)
            .expect("MCP config is valid JSON");

        if let Some(out_path) = output {
            match fs::write(&out_path, &config_str) {
                Ok(_) => println!("  {} 已保存到: {}", "✓".green(), out_path),
                Err(e) => println!("  {} 写入失败: {}", "✗".red(), e),
            }
        } else {
            println!("\n{}", config_str);
        }
    }

    // ── NEW: info command ──

    fn cmd_info(&self, section: Option<String>) {
        println!("\n{}", ">>> 配置说明".cyan().bold());

        match section.as_deref() {
            Some("server") => self.show_info_server(),
            Some("client") => self.show_info_client(),
            Some("security") => self.show_info_security(),
            _ => {
                // Show all sections
                self.show_info_server();
                println!();
                self.show_info_client();
                println!();
                self.show_info_security();
            }
        }
    }

    fn show_info_server(&self) {
        println!("\n  {}", "📦 服务端配置".yellow());
        println!("  {}", "─".repeat(40));
        println!("  服务端位于: {}", self.install_dir);
        println!("  配置文件: {}", self.env_file);
        println!("  配置项:");
        println!("    VLM_API_KEY      - 智谱 AI API Key");
        println!("    VLM_MODEL        - 模型名称 (默认: {})", DEFAULT_VLM_MODEL);
        println!("    VLM_ENDPOINT     - API 端点");
        println!("    DASHBOARD_PORT   - Dashboard 端口 (默认: 8000)");
        println!("    RUST_LOG         - 日志级别 (默认: info)");
        println!();
        println!("  常用命令:");
        println!("    pdf-mcp-cli config           - 配置 API Key");
        println!("    pdf-mcp-cli dashboard        - 启动 Web 管理界面");
        println!("    pdf-mcp-cli service start    - 启动服务");
        println!("    pdf-mcp-cli logs -f          - 查看日志");
    }

    fn show_info_client(&self) {
        println!("\n  {}", "🔗 客户端配置".yellow());
        println!("  {}", "─".repeat(40));
        println!("  客户端可以是任何 MCP 兼容的 AI 工具:");
        println!("    - Cursor IDE");
        println!("    - Claude Desktop");
        println!("    - Trae IDE");
        println!();
        println!("  客户端 MCP JSON 配置:");
        let config_example = serde_json::json!({
            "mcpServers": {
                "pdf-module": {
                    "command": format!("{}/pdf-mcp", self.install_dir),
                    "env": {
                        "VLM_API_KEY": "<your-api-key>",
                        "VLM_MODEL": DEFAULT_VLM_MODEL,
                        "VLM_ENDPOINT": DEFAULT_VLM_ENDPOINT
                    }
                }
            }
        });
        println!("  {}", serde_json::to_string_pretty(&config_example)
            .expect("config example is valid JSON")
            .replace('\n', "\n  "));
        println!();
        println!("  使用 generate-config 命令生成配置文件");
    }

    fn show_info_security(&self) {
        println!("\n  {}", "🔒 安全最佳实践".yellow());
        println!("  {}", "─".repeat(40));
        println!("  ✅ API Key 配置在服务端");
        println!("     - 集中管理敏感信息");
        println!("     - 不暴露给客户端");
        println!("     - 更新只需修改一处");
        println!();
        println!("  ✅ 客户端只需要服务地址");
        println!("     - 不要将 API Key 放在客户端 MCP JSON 中");
        println!("     - 避免泄露给其他人");
        println!("     - 避免提交到 Git 仓库");
        println!();
        println!("  ✅ 文件权限");
        println!("     - .env.local 文件权限建议设为 600");
        println!("     - 安装目录建议 root 所有");
    }

    // ── NEW: service command ──

    fn cmd_service(&self, action: ServiceAction) {
        match action {
            ServiceAction::Start => self.cmd_service_start(),
            ServiceAction::Stop => self.cmd_stop(),
            ServiceAction::Restart => {
                self.cmd_stop();
                std::thread::sleep(std::time::Duration::from_millis(500));
                self.cmd_service_start();
            }
            ServiceAction::Status => self.cmd_status(),
        }
    }

    fn cmd_service_start(&self) {
        println!("\n{}", ">>> 启动服务".cyan().bold());

        let config = self.load_config();

        // Check dashboard binary
        let dashboard_binary = format!("{}/pdf-dashboard", self.install_dir);
        if !Path::new(&dashboard_binary).exists() {
            println!("  {} pdf-dashboard 不存在", "✗".red());
            println!("  {} 请检查安装是否完整", "ℹ".blue());
            return;
        }

        if self.check_dashboard().is_some() {
            println!("  {} Dashboard API 已在运行", "ℹ".blue());
            return;
        }

        print!("  {} 启动 Dashboard API...", "→".blue());

        let pdfium_lib = format!("{}/lib/libpdfium.so", self.install_dir);
        let lib_dir = format!("{}/lib", self.install_dir);

        let result = Command::new(&dashboard_binary)
            .args(["--port", &config.dashboard_port.to_string()])
            .current_dir(&self.install_dir)
            .env("PDFIUM_LIB_PATH", &pdfium_lib)
            .env("LD_LIBRARY_PATH", &lib_dir)
            .env("VLM_API_KEY", &config.vlm_api_key)
            .env("VLM_MODEL", &config.vlm_model)
            .env("VLM_ENDPOINT", &config.vlm_endpoint)
            .env("RUST_LOG", &config.rust_log)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match result {
            Ok(child) => {
                self.save_pid(child.id());
                std::thread::sleep(std::time::Duration::from_millis(1000));

                if self.check_dashboard().is_some() {
                    println!(" {}", "✓".green());
                    println!("\n  {} 访问地址: http://localhost:{}", "→".blue(), config.dashboard_port);
                } else {
                    println!(" {}", "✗ 启动失败".red());
                }
            }
            Err(e) => {
                println!(" {} {}", "✗".red(), e);
            }
        }
    }

    // ── NEW: dashboard command ──

    fn cmd_dashboard(&self, port: u16) {
        println!("\n{}", ">>> 启动 Dashboard".cyan().bold());

        let config = self.load_config();
        let dashboard_binary = format!("{}/pdf-dashboard", self.install_dir);

        if !Path::new(&dashboard_binary).exists() {
            println!("  {} pdf-dashboard 不存在", "✗".red());
            println!("  {} 请检查安装是否完整", "ℹ".blue());
            return;
        }

        let actual_port = if port != 8000 { port } else { config.dashboard_port };

        if self.check_dashboard().is_some() {
            println!("  {} Dashboard 已在运行", "ℹ".blue());
            println!("  {} 访问: http://localhost:{}", "→".blue(), actual_port);
            return;
        }

        print!("  {} 启动 Dashboard (端口: {})...", "→".blue(), actual_port);

        let pdfium_lib = format!("{}/lib/libpdfium.so", self.install_dir);
        let lib_dir = format!("{}/lib", self.install_dir);

        let result = Command::new(&dashboard_binary)
            .args(["--port", &actual_port.to_string()])
            .current_dir(&self.install_dir)
            .env("PDFIUM_LIB_PATH", &pdfium_lib)
            .env("LD_LIBRARY_PATH", &lib_dir)
            .env("VLM_API_KEY", &config.vlm_api_key)
            .env("VLM_MODEL", &config.vlm_model)
            .env("VLM_ENDPOINT", &config.vlm_endpoint)
            .env("RUST_LOG", &config.rust_log)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match result {
            Ok(child) => {
                self.save_pid(child.id());
                std::thread::sleep(std::time::Duration::from_millis(1000));

                if self.check_dashboard().is_some() {
                    println!(" {}", "✓".green());
                    println!("\n  {} Dashboard 已启动", "→".blue());
                    println!("  {} 访问地址: http://localhost:{}", "→".blue(), actual_port);
                } else {
                    println!(" {}", "✗ 启动失败".red());
                }
            }
            Err(e) => {
                println!(" {} {}", "✗".red(), e);
            }
        }
    }

    // ── Legacy start (kept for backward compat) ──

    fn cmd_start_legacy(&self, web: bool) {
        if !web {
            println!("  {} MCP Server 按需启动，无需手动管理", "ℹ".blue());
            println!("  提示: 使用 `dashboard` 或 `service start` 命令");
            return;
        }
        self.cmd_service_start();
    }

    fn cmd_stop(&self) {
        println!("\n{}", ">>> 停止服务".cyan().bold());

        print!("  {} 停止 Dashboard API...", "→".blue());
        if let Some(pid) = self.check_dashboard() {
            let _ = Command::new("kill").args([&pid.to_string()]).status();
            std::thread::sleep(std::time::Duration::from_millis(500));

            if self.check_dashboard().is_none() {
                println!(" {}", "✓".green());
            } else {
                println!(" {}", "✗ 停止失败".red());
            }
        } else {
            println!(" {}", "未运行".blue());
        }

        let _ = fs::remove_file(&self.pid_file);
    }

    fn cmd_restart_legacy(&self) {
        self.cmd_stop();
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.cmd_service_start();
    }

    // ── NEW: upload command ──

    fn cmd_upload(&self, file_path: &str) {
        println!("\n{}", ">>> 上传 PDF 文件".cyan().bold());

        let path = Path::new(file_path);
        if !path.exists() {
            println!("  {} 文件不存在: {}", "✗".red(), file_path);
            return;
        }

        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "document.pdf".to_string());

        let file_content = match fs::read(path) {
            Ok(c) => c,
            Err(e) => {
                println!("  {} 读取文件失败: {}", "✗".red(), e);
                return;
            }
        };

        print!("  {} 上传 {}...", "→".blue(), file_name);

        let part = multipart::Part::bytes(file_content)
            .file_name(file_name.clone())
            .mime_str("application/pdf")
            .expect("application/pdf is a valid MIME type");

        let form = multipart::Form::new().part("file", part);

        let client = reqwest::blocking::Client::new();
        let upload_url = format!("{}/api/upload", self.server_url);

        match client.post(&upload_url).multipart(form).send() {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!(" {}", "✓".green());
                    println!("  {} 文件: {}", "→".blue(), file_name);
                    println!("  {} 服务器: {}", "→".blue(), self.server_url);
                } else {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    println!(" {}", "✗".red());
                    println!("  {} HTTP {}: {}", "✗".red(), status, body);
                }
            }
            Err(e) => {
                println!(" {}", "✗".red());
                println!("  {} 连接失败: {}", "✗".red(), e);
                println!("  {} 请确认 Dashboard 正在运行 (端口: {})", "ℹ".blue(), self.server_url);
            }
        }
    }

    // ── NEW: list command ──

    fn cmd_list(&self) {
        println!("\n{}", ">>> PDF 文件列表".cyan().bold());

        let client = reqwest::blocking::Client::new();
        let list_url = format!("{}/api/pdfs", self.server_url);

        match client.get(&list_url).send() {
            Ok(resp) => {
                if !resp.status().is_success() {
                    println!("  {} 请求失败: HTTP {}", "✗".red(), resp.status());
                    return;
                }

                let files: Vec<PdfFileInfo> = match resp.json() {
                    Ok(f) => f,
                    Err(_) => {
                        println!("  {} 无法解析服务器响应", "✗".red());
                        return;
                    }
                };

                if files.is_empty() {
                    println!("  {} 暂无 PDF 文件", "ℹ".blue());
                    return;
                }

                println!(
                    "\n  {:<4} {:<40} {:<10} {}",
                    "序号".cyan(),
                    "文件名".cyan(),
                    "大小".cyan(),
                    "页数".cyan()
                );
                println!("  {}", "─".repeat(70));

                for (i, file) in files.iter().enumerate() {
                    let size_str = if file.size > 1024 * 1024 {
                        format!("{:.1} MB", file.size as f64 / (1024.0 * 1024.0))
                    } else if file.size > 1024 {
                        format!("{:.1} KB", file.size as f64 / 1024.0)
                    } else {
                        format!("{} B", file.size)
                    };

                    let pages_str = file
                        .pages
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "  {:<4} {:<40} {:<10} {}",
                        i + 1,
                        truncate_str(&file.name, 38),
                        size_str,
                        pages_str
                    );
                }
                println!("  {} 共 {} 个文件", "→".blue(), files.len());
            }
            Err(e) => {
                println!("  {} 连接失败: {}", "✗".red(), e);
                println!("  {} 请确认 Dashboard 正在运行", "ℹ".blue());
            }
        }
    }

    // ── NEW: test command ──

    fn cmd_test(&self, file: Option<String>) {
        println!("\n{}", ">>> 测试 PDF 处理".cyan().bold());

        let test_path = match file {
            Some(p) => p,
            None => {
                print!("  请输入 PDF 文件路径 (直接回车使用默认): ");
                use std::io::{self, Write};
                io::stdout().flush().expect("stdout flush failed");
                let mut input = String::new();
                io::stdin().read_line(&mut input).expect("failed to read stdin");
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    println!("  {} 未提供文件路径", "ℹ".blue());
                    println!("  用法: pdf-mcp-cli test --file <path>");
                    return;
                }
                trimmed.to_string()
            }
        };

        let path = Path::new(&test_path);
        if !path.exists() {
            println!("  {} 文件不存在: {}", "✗".red(), test_path);
            return;
        }

        println!("  {} 测试文件: {}", "→".blue(), test_path);

        // Try to test via dashboard API
        let client = reqwest::blocking::Client::new();
        let test_url = format!("{}/api/test", self.server_url);
        let body = serde_json::json!({ "file": test_path });

        match client.post(&test_url).json(&body).send() {
            Ok(resp) => {
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().unwrap_or_default();
                    println!("  {} 测试结果: {}", "✓".green(), serde_json::to_string_pretty(&result)
                        .expect("test result is valid JSON"));
                } else {
                    let status = resp.status();
                    let body_text = resp.text().unwrap_or_default();
                    println!("  {} 测试失败: HTTP {} - {}", "✗".red(), status, body_text);
                }
            }
            Err(_) => {
                // Fallback: try direct MCP stdio test
                println!("  {} Dashboard 未连接，尝试直接测试...", "ℹ".blue());

                let mcp_binary = format!("{}/pdf-mcp", self.install_dir);
                if !Path::new(&mcp_binary).exists() {
                    println!("  {} pdf-mcp 未找到: {}", "✗".red(), mcp_binary);
                    return;
                }

                println!("  {} 测试: 获取页数", "→".blue());
                let mcp_request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "tools/call",
                    "params": {
                        "name": "get_page_count",
                        "arguments": { "file_path": test_path }
                    }
                });

                let mcp_str = serde_json::to_string(&mcp_request)
                    .expect("MCP request is valid JSON");
                let output = Command::new(&mcp_binary)
                    .arg("--stdio")
                    .arg("--single")
                    .arg(&mcp_str)
                    .env("PDFIUM_LIB_PATH", format!("{}/lib/libpdfium.so", self.install_dir))
                    .env("LD_LIBRARY_PATH", format!("{}/lib", self.install_dir))
                    .output();

                match output {
                    Ok(out) => {
                        let stdout_str = String::from_utf8_lossy(&out.stdout);
                        if out.status.success() {
                            println!("  {} 响应: {}", "✓".green(), stdout_str.trim());
                        } else {
                            let stderr_str = String::from_utf8_lossy(&out.stderr);
                            println!("  {} 错误: {}", "✗".red(), stderr_str.trim());
                        }
                    }
                    Err(e) => {
                        println!("  {} 执行失败: {}", "✗".red(), e);
                    }
                }
            }
        }
    }

    fn cmd_logs(&self, lines: u16, follow: bool) {
        let log_file = format!("{}/logs/latest.log", self.install_dir);

        if !Path::new(&log_file).exists() {
            println!("  {} 日志文件不存在: {}", "✗".red(), log_file);
            return;
        }

        if follow {
            let _ = Command::new("tail")
                .args(["-f", "-n", &lines.to_string(), &log_file])
                .status();
        } else {
            let _ = Command::new("tail")
                .args(["-n", &lines.to_string(), &log_file])
                .status();
        }
    }

    fn show_config_summary(&self, config: &EnvConfig) {
        println!("\n  {}", "配置摘要:".yellow());
        if config.vlm_api_key.is_empty() {
            println!("    {} API Key: 未配置", "✗".red());
        } else {
            let masked = format!(
                "{}****",
                &config.vlm_api_key[..8.min(config.vlm_api_key.len())]
            );
            println!("    {} API Key: {}", "✓".green(), masked);
        }
        println!("    {} 模型: {}", "→".blue(), config.vlm_model);
        println!("    {} 端点: {}", "→".blue(), config.vlm_endpoint);
    }

    // ── Interactive mode ──

    fn interactive_menu(&self) {
        use std::io::{self, Write};

        loop {
            let config = self.load_config();

            println!("\n  {}", "📋 主菜单".cyan().bold());
            println!("  {}", "─".repeat(34));

            // Check API key status for display
            let config_status = if config.vlm_api_key.is_empty() {
                "未配置".red().to_string()
            } else {
                "已配置".green().to_string()
            };

            println!("  {} 初始化配置    [{}]", " 1".cyan(), "init".blue());
            println!("  {} 配置 GLM API  [{}]", " 2".cyan(), config_status);
            println!("  {} 查看状态      [{}]", " 3".cyan(), "status".blue());
            println!("  {} 生成客户端配置 [{}]", " 4".cyan(), "generate-config".blue());
            println!("  {} 配置说明      [{}]", " 5".cyan(), "info".blue());
            println!("  {} 管理系统服务  [{}]", " 6".cyan(), "service".blue());
            println!("  {} 查看日志      [{}]", " 7".cyan(), "logs".blue());
            println!("  {} 启动 Dashboard [{}]", " 8".cyan(), "dashboard".blue());
            println!("  {} 上传 PDF      [{}]", " 9".cyan(), "upload".blue());
            println!("  {} 测试 PDF 处理 [{}]", "10".cyan(), "test".blue());
            println!("  {} 退出", " 0".cyan());

            print!("\n  选择: ");
            io::stdout().flush().expect("stdout flush failed");

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }

            match input.trim() {
                "1" => self.cmd_init(),
                "2" => self.cmd_config_interactive(),
                "3" => self.cmd_status(),
                "4" => self.cmd_generate_config(None),
                "5" => self.cmd_info(None),
                "6" => self.interactive_service_menu(),
                "7" => {
                    print!("  行数 [20]: ");
                    io::stdout().flush().expect("stdout flush failed");
                    let mut lines = String::new();
                    io::stdin().read_line(&mut lines).expect("failed to read stdin");
                    let n: u16 = lines.trim().parse().unwrap_or(20);
                    self.cmd_logs(n, false);
                }
                "8" => {
                    print!("  Dashboard 端口 [8000]: ");
                    io::stdout().flush().expect("stdout flush failed");
                    let mut port = String::new();
                    io::stdin().read_line(&mut port).expect("failed to read stdin");
                    let p: u16 = port.trim().parse().unwrap_or(8000);
                    self.cmd_dashboard(p);
                }
                "9" => {
                    print!("  PDF 文件路径: ");
                    io::stdout().flush().expect("stdout flush failed");
                    let mut path = String::new();
                    io::stdin().read_line(&mut path).expect("failed to read stdin");
                    let path = path.trim();
                    if !path.is_empty() {
                        self.cmd_upload(path);
                    }
                }
                "10" => self.cmd_test(None),
                "0" | "q" | "quit" | "exit" => {
                    println!("\n  再见！\n");
                    break;
                }
                _ => {}
            }

            if input.trim() != "0" && input.trim() != "q" {
                println!("\n  {} 按回车键继续...", "→".blue());
                let mut _pause = String::new();
                io::stdin().read_line(&mut _pause).ok();
            }
        }
    }

    fn cmd_config_interactive(&self) {
        use std::io::{self, Write};

        println!("\n{}", ">>> API 配置".cyan().bold());

        loop {
            let config = self.load_config();

            println!("\n  {}", "当前配置:".yellow());
            if config.vlm_api_key.is_empty() {
                println!("    {} API Key: {}", "✗".red(), "未配置".red());
            } else {
                let masked = format!(
                    "{}****",
                    &config.vlm_api_key[..8.min(config.vlm_api_key.len())]
                );
                println!("    {} API Key: {}", "✓".green(), masked);
            }
            println!("    {} 模型: {}", "→".blue(), config.vlm_model);

            println!("\n  {} 配置 API Key", "1".cyan());
            println!("  {} 配置模型", "2".cyan());
            println!("  {} 返回", "0".cyan());
            print!("\n  选择: ");
            io::stdout().flush().expect("stdout flush failed");

            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("failed to read stdin");

            match input.trim() {
                "1" => {
                    println!("\n  获取 API Key: https://open.bigmodel.cn/");
                    print!("  输入 API Key: ");
                    io::stdout().flush().expect("stdout flush failed");

                    let mut key = String::new();
                    io::stdin().read_line(&mut key).expect("failed to read stdin");

                    if !key.trim().is_empty() {
                        let mut config = self.load_config();
                        config.vlm_api_key = key.trim().to_string();
                        self.save_config(&config).ok();
                        println!("  {} 已保存", "✓".green());
                    }
                }
                "2" => {
                    let models = &["glm-4v-flash (推荐)", "glm-4v-plus"];
                    let selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("选择模型")
                        .items(models)
                        .default(0)
                        .interact()
                        .expect("failed to select model");

                    let model = match selection {
                        1 => "glm-4v-plus",
                        _ => "glm-4v-flash",
                    };

                    let mut config = self.load_config();
                    config.vlm_model = model.to_string();
                    self.save_config(&config).ok();
                    println!("  {} 模型: {}", "✓".green(), model);
                }
                "0" | "" => break,
                _ => {}
            }
        }
    }

    fn interactive_service_menu(&self) {
        use std::io::{self, Write};

        loop {
            println!("\n  {}", "🔧 服务管理".cyan().bold());
            println!("  {}", "─".repeat(20));
            println!("  {} 启动服务", "1".cyan());
            println!("  {} 停止服务", "2".cyan());
            println!("  {} 重启服务", "3".cyan());
            println!("  {} 查看状态", "4".cyan());
            println!("  {} 返回主菜单", "0".cyan());
            print!("\n  选择: ");
            io::stdout().flush().expect("stdout flush failed");

            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("failed to read stdin");

            match input.trim() {
                "1" => self.cmd_service_start(),
                "2" => self.cmd_stop(),
                "3" => {
                    self.cmd_stop();
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    self.cmd_service_start();
                }
                "4" => self.cmd_status(),
                "0" | "" => break,
                _ => {}
            }
        }
    }
}

// ─────────────────────────── Helper ───────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

// ─────────────────────────── Main ───────────────────────────

fn main() {
    let cli = Cli::parse();
    let server_url = cli.server.clone();
    let manager = McpManager::new(None, Some(server_url));

    // No command → interactive mode
    if cli.command.is_none() {
        manager.show_banner();
        manager.interactive_menu();
        return;
    }

    manager.show_banner();

    match cli.command {
        None => {}

        // Core commands
        Some(Commands::Init) => manager.cmd_init(),
        Some(Commands::Config {
            key,
            model,
            endpoint,
        }) => manager.cmd_config(key, model, endpoint),
        Some(Commands::Status) => manager.cmd_status(),
        Some(Commands::GenerateConfig { output }) => manager.cmd_generate_config(output),

        // NEW: info
        Some(Commands::Info { section }) => manager.cmd_info(section),

        // NEW: service subcommand
        Some(Commands::Service { action }) => manager.cmd_service(action),

        // NEW: dashboard
        Some(Commands::Dashboard { port }) => manager.cmd_dashboard(port),

        // NEW: interactive
        Some(Commands::Interactive) => {
            manager.interactive_menu();
        }

        // NEW: file operations
        Some(Commands::Upload { file }) => manager.cmd_upload(&file),
        Some(Commands::List) => manager.cmd_list(),
        Some(Commands::Test { file }) => manager.cmd_test(file),

        // Legacy commands
        Some(Commands::Start { web }) => manager.cmd_start_legacy(web),
        Some(Commands::Stop) => manager.cmd_stop(),
        Some(Commands::Restart) => manager.cmd_restart_legacy(),
        Some(Commands::Logs { lines, follow }) => manager.cmd_logs(lines, follow),
        Some(Commands::Ps) => manager.cmd_ps(),
    }
}
