mod config;
mod provider_panel;

use clap::{Parser, Subcommand};
use colored::Colorize;
use omega_agent_lib::{commands, pipeline, AppState, TerminalPrinter, default_db_path};
use serde::{Deserialize, Serialize};
use std::io::{stdin, stdout, Write};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    if !cli.cli {
        omega_agent_lib::run();
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()?;

    match cli.command {
        None => {
            rt.block_on(repl())?;
        }
        Some(Command::Chat { message }) => {
            rt.block_on(cmd_chat(message))?;
        }
        Some(Command::Plan { task }) => {
            rt.block_on(cmd_plan(task))?;
        }
        Some(Command::Code { task, execute }) => {
            rt.block_on(cmd_code(task, execute))?;
        }
        Some(Command::PlanStatus) => {
            rt.block_on(cmd_plan_status())?;
        }
        Some(Command::PlanApprove) => {
            rt.block_on(cmd_plan_approve())?;
        }
        Some(Command::Build { auto_approve }) => {
            rt.block_on(cmd_build(auto_approve))?;
        }
        Some(Command::Review { file, context }) => {
            rt.block_on(cmd_review(file, context))?;
        }
        Some(Command::Gate { file }) => {
            rt.block_on(cmd_gate(file))?;
        }
        Some(Command::Memory(command)) => {
            rt.block_on(cmd_memory(command))?;
        }
        Some(Command::Config(command)) => {
            rt.block_on(cmd_config(command))?;
        }
        Some(Command::ListModels { provider }) => {
            rt.block_on(cmd_models(provider, None, None))?;
        }
        Some(Command::Provider) => {
            rt.block_on(cmd_provider())?;
        }
        Some(Command::Models { provider, base_url, api_key }) => {
            rt.block_on(cmd_models(provider, base_url, api_key))?;
        }
    }

    Ok(())
}

// ─── Clap CLI ─────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "omega", version, about = "Omega Agent CLI — AI coding assistant")]
struct Cli {
    /// Run in CLI mode (default: true)
    #[arg(long, global = true, default_value_t = true)]
    cli: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Send a message to the LLM (streaming output)
    Chat {
        /// Message to send
        message: String,
    },
    /// Generate a structured plan from a task description
    Plan {
        /// Task description
        task: String,
    },
    /// Generate a code plan and optionally execute it
    #[command(alias = "c")]
    Code {
        /// Task description
        task: String,
        /// Execute the generated plan after approval
        #[arg(short, long)]
        execute: bool,
    },
    /// View the current plan
    PlanStatus,
    /// Approve the current plan for building
    PlanApprove,
    /// Execute build from the approved plan
    Build {
        /// Auto-approve all permission requests
        #[arg(long)]
        auto_approve: bool,
    },
    /// Run Gate + LLM review on a file
    Review {
        /// File path to review
        file: String,
        /// Optional context string
        context: Option<String>,
    },
    /// Run Gate checks only on a file
    Gate {
        /// File path to check
        file: String,
    },
    /// Memory operations
    #[command(subcommand)]
    Memory(MemorySubcommand),
    /// Configuration
    #[command(subcommand)]
    Config(ConfigSubcommand),
    /// List available models for a provider (static, use `omega models` to fetch live)
    ListModels {
        /// Provider name (optional, defaults to current)
        provider: Option<String>,
    },
    /// Interactive provider setup — select provider, enter API key, fetch models, select model
    Provider,
    /// Fetch and list models from the configured provider
    Models {
        /// Provider name (optional, defaults to current)
        provider: Option<String>,
        /// Custom base URL (hidden from output)
        #[arg(long)]
        base_url: Option<String>,
        /// API key (hidden from output)
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[derive(Subcommand)]
enum MemorySubcommand {
    /// Store a value in memory
    Store { key: String, value: String, #[arg(long, default_value = "session")] layer: String },
    /// Search memory
    Search { query: String, #[arg(long)] layer: Option<String>, #[arg(long, default_value_t = 10)] limit: usize },
    /// Recall a value by key
    Remember { key: String, #[arg(long)] layer: Option<String> },
    /// Count memory entries
    Count { #[arg(long)] layer: Option<String> },
    /// Delete a memory entry by ID
    Delete { id: String },
    /// Clear memory entries
    Clear { #[arg(long)] layer: Option<String> },
}

#[derive(Subcommand)]
enum ConfigSubcommand {
    /// Show current configuration
    Show,
    /// Set a configuration value (e.g. model, base_url, api_key, provider)
    Set { key: String, value: String },
    /// List available providers
    Providers,
}

// ─── Shared state ──────────────────────────────────────────────────────────────

fn create_state() -> AppState {
    let db_path = default_db_path();
    let provider_config = config::load_provider_config();
    AppState::new_with_provider_config(&db_path, provider_config)
}

fn create_state_with_provider(provider_config: providers::ProviderConfig) -> AppState {
    let db_path = default_db_path();
    AppState::new_with_provider_config(&db_path, provider_config)
}

// ─── Banner & Formatting ───────────────────────────────────────────────────────

const BANNER: &str = r#"        %@#%%%%%%#%@
     %#@@@@@@@@@@@@%#
   #@%#%%%%%%%%%%%%#%@
   %@%#  OMEGA AGENT #@%
   #@%#%%%%%%%%%%%%#%@
     %#@@@@@@@@@@@@%#
        %@#%%%%%%#%@"#;

fn print_banner(cfg: &providers::ProviderConfig) {
    println!("{}", BANNER.bold());
    println!();
    println!("   provider: {}", cfg.kind.to_string());
    println!("   model:    {}", cfg.model);
    println!();
}

fn print_section(title: &str) {
    println!("   {} {}", "──", title.bold().underline());
    println!();
}

fn print_success(message: &str) {
    println!("   {}", message);
}

fn print_error(message: &str) {
    eprintln!("   {} {}", "ERROR".bold(), message);
}

fn print_error_detail(message: &str, detail: &str) {
    eprintln!("   {} {}", "ERROR".bold(), message);
    if !detail.is_empty() {
        eprintln!("   {}", detail.dimmed());
    }
}

// ─── Interactive helpers ───────────────────────────────────────────────────────

fn prompt_api_key() -> anyhow::Result<String> {
    print!("   Enter API key: ");
    stdout().flush()?;
    let key = rpassword::read_password()
        .map_err(|e| anyhow::anyhow!("Failed to read API key: {}", e))?;
    Ok(key.trim().to_string())
}

fn prompt_base_url(kind: &providers::ProviderKind) -> anyhow::Result<Option<String>> {
    let _ = kind.default_base_url();
    print!("   Custom base URL? [Enter for default]: ");
    stdout().flush()?;
    let url = rpassword::read_password()
        .map_err(|e| anyhow::anyhow!("Failed to read base URL: {}", e))?;
    let trimmed = url.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn print_models(models: &[providers::ModelInfo]) {
    print_section("AVAILABLE MODELS");
    println!("   #    MODEL");
    for (i, m) in models.iter().enumerate() {
        println!("   {:<4} {}", i + 1, m.display_name());
    }
    println!();
}

fn select_model(models: &[providers::ModelInfo]) -> anyhow::Result<String> {
    print_models(models);
    print!("   Select model [number or ID]: ");
    stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty() {
        anyhow::bail!("No model selected");
    }

    if let Ok(idx) = trimmed.parse::<usize>() {
        if idx >= 1 && idx <= models.len() {
            return Ok(models[idx - 1].id.clone());
        }
    }

    if models.iter().any(|m| m.id == trimmed || m.name.as_deref() == Some(trimmed)) {
        return Ok(trimmed.to_string());
    }

    anyhow::bail!("Invalid model selection")
}

async fn ensure_model_ready(cfg: &mut providers::ProviderConfig) -> anyhow::Result<()> {
    let base_url = cfg.base_url.clone().unwrap_or_else(|| cfg.kind.default_base_url());

    if cfg.model.is_empty() || cfg.model == "gpt-4" {
        println!("   Fetching models...");
        let temp_cfg = providers::ProviderConfig {
            base_url: Some(base_url.clone()),
            api_key: cfg.api_key.clone(),
            kind: cfg.kind.clone(),
            ..providers::ProviderConfig::default()
        };

        match providers::fetch_models(&temp_cfg).await {
            Ok(models) => {
                let selected = select_model(&models)?;
                cfg.model = selected.clone();
                let mut cli_config = config::load_config();
                cli_config.model = Some(selected);
                let _ = config::save_config(&cli_config);
            }
            Err(e) => {
                print_error_detail("Could not fetch models.", &e);
                return Err(anyhow::anyhow!("Model fetch failed"));
            }
        }
    }
    Ok(())
}

// ─── Command handlers ──────────────────────────────────────────────────────────

async fn cmd_provider() -> anyhow::Result<()> {
    let provider_cfg = config::load_provider_config();

    // Step 1: Ratatui provider selection panel
    let kind = match provider_panel::select_provider(&provider_cfg.kind)? {
        Some(k) => k,
        None => return Ok(()),
    };

    println!();
    print_success(&format!("Selected provider: {}", kind));

    // Step 2: API key and base URL via rpassword
    let api_key = prompt_api_key()?;
    let base_url = prompt_base_url(&kind)?;

    println!();
    println!("   Fetching models...");

    // Step 3: Ratatui model fetch spinner + selection
    let temp_cfg = providers::ProviderConfig {
        base_url: base_url.clone().or_else(|| Some(kind.default_base_url())),
        api_key: Some(api_key.clone()),
        kind: kind.clone(),
        model: String::new(),
        ..providers::ProviderConfig::default()
    };

    match provider_panel::select_model(&temp_cfg).await? {
        Some(model_id) => {
            let cli_cfg = config::CliConfig {
                provider: Some(kind.to_string()),
                model: Some(model_id),
                base_url,
                permission_mode: config::load_config().permission_mode,
            };
            config::save_config(&cli_cfg).map_err(|e| anyhow::anyhow!(e))?;
            config::save_api_key(&api_key).map_err(|e| anyhow::anyhow!(e))?;

            println!();
            print_success("Configuration saved.");
        }
        None => {
            println!();
            println!("   Model selection cancelled.");
        }
    }

    Ok(())
}

async fn cmd_models(
    provider: Option<String>,
    base_url_override: Option<String>,
    api_key_override: Option<String>,
) -> anyhow::Result<()> {
    let mut cfg = config::load_provider_config();

    if let Some(ref name) = provider {
        cfg.kind = providers::ProviderKind::from_str(name);
    }
    if let Some(ref url) = base_url_override {
        cfg.base_url = Some(url.clone());
    }
    if let Some(ref key) = api_key_override {
        cfg.api_key = Some(key.clone());
    }

    print_banner(&cfg);

    println!("   Fetching models...");

    match providers::fetch_models(&cfg).await {
        Ok(models) => {
            print_section("AVAILABLE MODELS");
            println!("   #    MODEL");
            for (i, m) in models.iter().enumerate() {
                println!("   {:<4} {}", i + 1, m.display_name());
            }
        }
        Err(_e) => {
            print_warning("Could not fetch models from provider. Showing built-in list.");

            let static_list = match cfg.kind {
                providers::ProviderKind::Anthropic => {
                    vec![
                        providers::ModelInfo { id: "claude-3-5-sonnet-20241022".into(), name: Some("Claude 3.5 Sonnet".into()), provider: "anthropic".into() },
                        providers::ModelInfo { id: "claude-3-5-haiku-20241022".into(), name: Some("Claude 3.5 Haiku".into()), provider: "anthropic".into() },
                        providers::ModelInfo { id: "claude-opus-4-20250514".into(), name: Some("Claude Opus 4".into()), provider: "anthropic".into() },
                    ]
                }
                providers::ProviderKind::Google => {
                    vec![
                        providers::ModelInfo { id: "models/gemini-1.5-flash".into(), name: Some("Gemini 1.5 Flash".into()), provider: "google".into() },
                        providers::ModelInfo { id: "models/gemini-1.5-pro".into(), name: Some("Gemini 1.5 Pro".into()), provider: "google".into() },
                        providers::ModelInfo { id: "models/gemini-2.0-flash-exp".into(), name: Some("Gemini 2.0 Flash".into()), provider: "google".into() },
                    ]
                }
                _ => {
                    vec![
                        providers::ModelInfo { id: "gpt-4o".into(), name: None, provider: cfg.kind.to_string() },
                        providers::ModelInfo { id: "gpt-4o-mini".into(), name: None, provider: cfg.kind.to_string() },
                        providers::ModelInfo { id: "gpt-4-turbo".into(), name: None, provider: cfg.kind.to_string() },
                    ]
                }
            };

            print_models(&static_list);
        }
    }

    Ok(())
}

fn print_warning(message: &str) {
    println!("   {} {}", "WARNING".bold(), message);
}

async fn cmd_chat(message: String) -> anyhow::Result<()> {
    let mut provider_cfg = config::load_provider_config();
    ensure_model_ready(&mut provider_cfg).await?;

    print_banner(&provider_cfg);
    print_section("CHAT");
    println!("   > {}", message);
    println!();

    let state = create_state_with_provider(provider_cfg.clone());
    let request = commands::chat::StreamMessageRequest {
        content: message,
        agent_type: "chat".into(),
        provider: Some(provider_cfg),
        system_prompt: None,
        permission_mode: config::load_config().permission_mode.unwrap_or_default(),
    };
    let emitter = TerminalPrinter::new();
    match commands::chat::stream_message(&state, request, &emitter).await {
        Ok(_) => {}
        Err(e) => {
            print_error_detail("LLM request failed.", &e);
        }
    }
    Ok(())
}

fn print_plan(plan: &pipeline::plan::StructuredPlan) {
    println!("{}", "─".repeat(60));
    println!("{} {}", "Task:".bold(), plan.task_summary);
    println!("{} {}", "Language:".bold(), plan.language);
    println!("{} {}", "Complexity:".bold(), plan.estimated_complexity);
    println!("{} {}", "Risk:".bold(), plan.risk_level);
    println!("{} {} file(s)", "Files affected:".bold(), plan.files_affected.len());
    println!();
    for step in &plan.steps {
        let status = if step.dependencies.is_empty() { "→" } else { "↳" };
        println!("  {} {} {} {}", status, format!("#{}", step.id).bold(), step.action, step.description);
        if let Some(ref fp) = step.file_path {
            println!("     file: {}", fp);
        }
    }
    println!("{}", "─".repeat(60));
}

async fn cmd_plan(task: String) -> anyhow::Result<()> {
    let state = create_state();
    match commands::plan_cmd::generate_plan(&state, task).await {
        Ok(payload) => {
            print_plan(&payload.plan);
        }
        Err(e) => eprintln!("{} {}", "Error:".bold(), e),
    }
    Ok(())
}

async fn cmd_code(task: String, execute: bool) -> anyhow::Result<()> {
    let state = create_state();
    match commands::plan_cmd::generate_plan(&state, task).await {
        Ok(payload) => {
            print_plan(&payload.plan);
            if execute {
                println!("\n{} {}", "Action:".bold(), "approve and execute plan");
                cmd_build(true).await?;
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".bold(), e),
    }
    Ok(())
}

async fn cmd_plan_status() -> anyhow::Result<()> {
    let state = create_state();
    match commands::plan_cmd::get_plan(&state).await {
        Ok(Some(plan)) => print_plan(&plan),
        Ok(None) => println!("{}", "No plan has been generated yet.".bold()),
        Err(e) => eprintln!("{} {}", "Error:".bold(), e),
    }
    Ok(())
}

async fn cmd_plan_approve() -> anyhow::Result<()> {
    let state = create_state();
    match commands::plan_cmd::approve_plan(&state).await {
        Ok(msg) => println!("{}", msg),
        Err(e) => eprintln!("{} {}", "Error:".bold(), e),
    }
    Ok(())
}

async fn cmd_build(auto_approve: bool) -> anyhow::Result<()> {
    let state = create_state();
    if auto_approve {
        let _ = commands::build_cmd::set_build_config(&state, true).await;
    }
    match commands::build_cmd::execute_build(&state).await {
        Ok(session) => {
            let completed = session.iter().filter(|e| e.success).count();
            let failed = session.iter().filter(|e| !e.success).count();
            println!();
            println!("{} {} succeeded, {} failed", "Build complete:".bold(), completed, failed);
            for entry in &session {
                let icon = if entry.success { "✓" } else { "✗" };
                println!("  {} step {} ({}): {}ms", icon, entry.step_index, entry.tool, entry.duration_ms);
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".bold(), e),
    }
    if auto_approve {
        let _ = commands::build_cmd::set_build_config(&state, false).await;
    }
    Ok(())
}

async fn cmd_review(file: String, context: Option<String>) -> anyhow::Result<()> {
    let code = std::fs::read_to_string(&file)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file, e))?;
    let ctx = context.unwrap_or_default();
    let state = create_state();
    let request = commands::review_cmd::ReviewRequest { code, context: ctx };
    match commands::review_cmd::run_review(&state, request).await {
        Ok(output) => {
            println!("{} {}", "Gate Score:".bold(), output.score_breakdown.combined_score);
            println!("{} {}", "Passed:".bold(), if output.score_breakdown.passed { "Yes" } else { "No" });
            if !output.gate_violations.is_empty() {
                println!("\n{}", "Gate Violations:".bold());
                for v in &output.gate_violations {
                    println!("  [{}] {} — {}", v.category, v.message, v.tool_hint.as_deref().unwrap_or(""));
                }
            }
            if let Some(ref llm) = output.llm_review {
                println!("\n{}", "LLM Review:".bold());
                println!("{}", llm);
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".bold(), e),
    }
    Ok(())
}

async fn cmd_gate(file: String) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&file)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file, e))?;
    let state = create_state();
    let request = commands::gate::GateCheckRequest {
        content,
        context: format!("reviewing {}", file),
        language: None,
    };
    match commands::gate::check_gate(&state, request).await {
        Ok(result) => {
            println!("{} {}", "Gate Score:".bold(), result.score);
            println!("{} {}", "Passed:".bold(), if result.passed { "Yes" } else { "No" });
            for v in &result.violations {
                println!("  [{}] {} — {}", v.category.bold(), v.message, v.tool_hint.as_deref().unwrap_or(""));
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".bold(), e),
    }
    Ok(())
}

async fn cmd_memory(cmd: MemorySubcommand) -> anyhow::Result<()> {
    let state = create_state();
    match cmd {
        MemorySubcommand::Store { key, value, layer } => {
            let req = commands::memory::MemoryStoreRequest { key, value, layer };
            match commands::memory::memory_store(&state, req).await {
                Ok(msg) => println!("{}", msg),
                Err(e) => eprintln!("{} {}", "Error:".bold(), e),
            }
        }
        MemorySubcommand::Search { query, layer, limit } => {
            let req = commands::memory::MemorySearchRequest { query, layer, limit: Some(limit) };
            match commands::memory::memory_search(&state, req).await {
                Ok(resp) => {
                    if resp.entries.is_empty() {
                        println!("{}", "No results found.".bold());
                    } else {
                        for (i, entry) in resp.entries.iter().enumerate() {
                            let relevance = resp.relevance.get(i).copied().unwrap_or(0.0);
                            println!("  [{}] {} (relevance: {:.0}%)", entry.id, entry.key, relevance * 100.0);
                            println!("       layer: {:?} | value: {}", entry.layer, entry.value);
                        }
                    }
                }
                Err(e) => eprintln!("{} {}", "Error:".bold(), e),
            }
        }
        MemorySubcommand::Remember { key, layer } => {
            match commands::memory::memory_remember(&state, key, layer).await {
                Ok(Some(value)) => println!("{}", value),
                Ok(None) => println!("{}", "Not found.".bold()),
                Err(e) => eprintln!("{} {}", "Error:".bold(), e),
            }
        }
        MemorySubcommand::Count { layer } => {
            match commands::memory::memory_count(&state, layer).await {
                Ok(count) => println!("{} entries", count),
                Err(e) => eprintln!("{} {}", "Error:".bold(), e),
            }
        }
        MemorySubcommand::Delete { id } => {
            match commands::memory::memory_delete(&state, id).await {
                Ok(()) => println!("{}", "Deleted."),
                Err(e) => eprintln!("{} {}", "Error:".bold(), e),
            }
        }
        MemorySubcommand::Clear { layer } => {
            match commands::memory::memory_clear(&state, layer).await {
                Ok(count) => println!("Cleared {} entries.", count),
                Err(e) => eprintln!("{} {}", "Error:".bold(), e),
            }
        }
    }
    Ok(())
}

async fn cmd_config(cmd: ConfigSubcommand) -> anyhow::Result<()> {
    match cmd {
        ConfigSubcommand::Show => {
            let cfg = config::load_provider_config();
            print_banner(&cfg);
        }
        ConfigSubcommand::Set { key, value } => {
            let mut cli_cfg = config::load_config();
            let mut provider_cfg = cli_cfg.to_provider_config();

            match key.as_str() {
                "provider" => {
                    provider_cfg.kind = providers::ProviderKind::from_str(&value);
                    println!("   provider updated");
                }
                "model" => {
                    provider_cfg.model = value;
                    println!("   model updated");
                }
                "base_url" => {
                    provider_cfg.base_url = Some(value);
                    println!("   base_url updated");
                }
                "api_key" => {
                    config::save_api_key(&value).map_err(|e| anyhow::anyhow!(e))?;
                    println!("   api_key updated (stored in .env)");
                    return Ok(());
                }
                _ => {
                    print_error("Unknown key. Try: provider, model, base_url, api_key, max_tokens, temperature");
                    return Ok(());
                }
            }

            cli_cfg = config::CliConfig::from_provider_config(&provider_cfg);
            config::save_config(&cli_cfg).map_err(|e| anyhow::anyhow!(e))?;
        }
        ConfigSubcommand::Providers => {
            let cfg = config::load_provider_config();
            print_banner(&cfg);
            print_section("AVAILABLE PROVIDERS");
            for (i, kind) in providers::ProviderKind::all().iter().enumerate() {
                println!("   {:<4} {}", i + 1, kind);
            }
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
struct HistoryEntry {
    role: String,
    content: String,
}

fn sessions_dir() -> std::path::PathBuf {
    config::config_dir().join("sessions")
}

fn save_history(name: &str, history: &[HistoryEntry]) -> Result<(), String> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create sessions dir: {}", e))?;
    let path = dir.join(format!("{}.json", name));
    let json = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| format!("Failed to save session: {}", e))
}

fn load_history(name: &str) -> Result<Vec<HistoryEntry>, String> {
    let path = sessions_dir().join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path).map_err(|_| format!("Session '{}' not found", name))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse session: {}", e))
}

fn list_sessions() -> Result<Vec<String>, String> {
    let dir = sessions_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut sessions = vec![];
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to list sessions: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()) {
            sessions.push(name);
        }
    }
    sessions.sort();
    Ok(sessions)
}

// ─── REPL ──────────────────────────────────────────────────────────────────────

async fn repl() -> anyhow::Result<()> {
    let state = create_state();
    let cfg = config::load_provider_config();
    print_banner(&cfg);
    println!("   Interactive mode. Type /help.");
    println!();

    let mut last_message: Option<String> = None;
    let mut system_prompt: Option<String> = None;
    let mut history: Vec<HistoryEntry> = vec![];

    loop {
        print!("{} ", "#>".bold());
        let _ = stdout().flush();
        let mut line = String::new();
        if stdin().read_line(&mut line).is_err() || line.trim().is_empty() {
            continue;
        }
        let line = line.trim().to_string();

        if !line.starts_with('/') {
            let mut provider_cfg = config::load_provider_config();
            if let Err(e) = ensure_model_ready(&mut provider_cfg).await {
                print_error_detail("Model not ready.", &e.to_string());
                continue;
            }
            let state = create_state_with_provider(provider_cfg.clone());
            last_message = Some(line.clone());
            let request = commands::chat::StreamMessageRequest {
                content: line.clone(),
                agent_type: "chat".into(),
                provider: Some(provider_cfg.clone()),
                system_prompt: system_prompt.clone(),
                permission_mode: config::load_config().permission_mode.unwrap_or_default(),
            };
            let emitter = TerminalPrinter::new();
            let user_content = line;
            match commands::chat::stream_message(&state, request, &emitter).await {
                Ok(response) => {
                    history.push(HistoryEntry { role: "user".into(), content: user_content });
                    if !response.is_empty() {
                        history.push(HistoryEntry { role: "assistant".into(), content: response });
                    }
                }
                Err(e) => print_error_detail("LLM request failed.", &e),
            }
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let rest = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

        match cmd.as_str() {
            "/chat" | "/c" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /chat <message>", "Usage:".bold());
                    continue;
                }
                let mut provider_cfg = config::load_provider_config();
                if let Err(e) = ensure_model_ready(&mut provider_cfg).await {
                    print_error_detail("Model not ready.", &e.to_string());
                    continue;
                }
                let state = create_state_with_provider(provider_cfg.clone());
                last_message = Some(rest.clone());
                let request = commands::chat::StreamMessageRequest {
                    content: rest.clone(),
                    agent_type: "chat".into(),
                    provider: Some(provider_cfg.clone()),
                    system_prompt: system_prompt.clone(),
                    permission_mode: config::load_config().permission_mode.unwrap_or_default(),
                };
                let emitter = TerminalPrinter::new();
                let user_content = rest;
                match commands::chat::stream_message(&state, request, &emitter).await {
                    Ok(response) => {
                        history.push(HistoryEntry { role: "user".into(), content: user_content });
                        if !response.is_empty() {
                            history.push(HistoryEntry { role: "assistant".into(), content: response });
                        }
                    }
                    Err(e) => print_error_detail("LLM request failed.", &e),
                }
            }
            "/plan" | "/p" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /plan <task description>", "Usage:".bold());
                    continue;
                }
                match commands::plan_cmd::generate_plan(&state, rest).await {
                    Ok(payload) => print_plan(&payload.plan),
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/plan-status" => {
                match commands::plan_cmd::get_plan(&state).await {
                    Ok(Some(plan)) => print_plan(&plan),
                    Ok(None) => println!("{}", "No plan has been generated yet.".bold()),
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/plan-approve" | "/approve" => {
                match commands::plan_cmd::approve_plan(&state).await {
                    Ok(msg) => println!("{}", msg),
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/build" | "/b" => {
                let auto_approve = rest == "auto";
                if auto_approve {
                    let _ = commands::build_cmd::set_build_config(&state, true).await;
                }
                match commands::build_cmd::execute_build(&state).await {
                    Ok(session) => {
                        let completed = session.iter().filter(|e| e.success).count();
                        let failed = session.iter().filter(|e| !e.success).count();
                        println!("{} {} succeeded, {} failed", "Build complete:".bold(), completed, failed);
                    }
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
                if auto_approve {
                    let _ = commands::build_cmd::set_build_config(&state, false).await;
                }
            }
            "/review" | "/r" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /review <file> [context]", "Usage:".bold());
                    continue;
                }
                let (file_path, ctx) = match rest.split_once(' ') {
                    Some((f, c)) => (f.to_string(), c.to_string()),
                    None => (rest.clone(), String::new()),
                };
                let code = match std::fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{} Failed to read {}: {}", "Error:".bold(), file_path, e);
                        continue;
                    }
                };
                let request = commands::review_cmd::ReviewRequest { code, context: ctx };
                match commands::review_cmd::run_review(&state, request).await {
                    Ok(output) => {
                        println!("{} {}", "Gate Score:".bold(), output.score_breakdown.combined_score);
                        if !output.gate_violations.is_empty() {
                            println!("\n{}", "Gate Violations:".bold());
                            for v in &output.gate_violations {
                                println!("  [{}] {}", v.category, v.message);
                            }
                        }
                        if let Some(ref llm) = output.llm_review {
                            println!("\n{}", "LLM Review:".bold());
                            println!("{}", llm);
                        }
                    }
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/gate" | "/g" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /gate <file>", "Usage:".bold());
                    continue;
                }
                let content = match std::fs::read_to_string(&rest) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{} Failed to read {}: {}", "Error:".bold(), rest, e);
                        continue;
                    }
                };
                let request = commands::gate::GateCheckRequest {
                    content,
                    context: format!("reviewing {}", rest),
                    language: None,
                };
                match commands::gate::check_gate(&state, request).await {
                    Ok(result) => {
                        println!("{} {}", "Gate Score:".bold(), result.score);
                        for v in &result.violations {
                            println!("  [{}] {}", v.category.bold(), v.message);
                        }
                    }
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/memory" | "/m" => {
                let args: Vec<&str> = rest.splitn(4, ' ').collect();
                if args.is_empty() || args[0].is_empty() {
                    eprintln!("{} Usage: /memory <search|store|remember|count|delete|clear> ...", "Usage:".bold());
                    continue;
                }
                match args[0] {
                    "store" => {
                        if args.len() < 3 {
                            eprintln!("{} Usage: /memory store <key> <value> [layer]", "Usage:".bold());
                            continue;
                        }
                        let key = args[1];
                        let value = args[2];
                        let layer = args.get(3).copied().unwrap_or("session");
                        let req = commands::memory::MemoryStoreRequest { key: key.into(), value: value.into(), layer: layer.into() };
                        match commands::memory::memory_store(&state, req).await {
                            Ok(msg) => println!("{}", msg),
                            Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                        }
                    }
                    "search" => {
                        if args.len() < 2 {
                            eprintln!("{} Usage: /memory search <query> [layer] [limit]", "Usage:".bold());
                            continue;
                        }
                        let query = args[1];
                        let layer = args.get(2).copied().map(String::from);
                        let limit = args.get(3).and_then(|s| s.parse().ok());
                        let req = commands::memory::MemorySearchRequest { query: query.into(), layer, limit };
                        match commands::memory::memory_search(&state, req).await {
                            Ok(resp) => {
                                for (i, entry) in resp.entries.iter().enumerate() {
                                    let relevance = resp.relevance.get(i).copied().unwrap_or(0.0);
                                    println!("  [{}] {} ({:.0}%)", entry.id, entry.key, relevance * 100.0);
                                    println!("       layer: {:?} | value: {}", entry.layer, entry.value);
                                }
                            }
                            Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                        }
                    }
                    "remember" => {
                        if args.len() < 2 {
                            eprintln!("{} Usage: /memory remember <key>", "Usage:".bold());
                            continue;
                        }
                        match commands::memory::memory_remember(&state, args[1].into(), None).await {
                            Ok(Some(value)) => println!("{}", value),
                            Ok(None) => println!("{}", "Not found.".bold()),
                            Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                        }
                    }
                    "count" => {
                        match commands::memory::memory_count(&state, None).await {
                            Ok(count) => println!("{} entries", count),
                            Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                        }
                    }
                    "delete" => {
                        if args.len() < 2 {
                            eprintln!("{} Usage: /memory delete <id>", "Usage:".bold());
                            continue;
                        }
                        match commands::memory::memory_delete(&state, args[1].into()).await {
                            Ok(()) => println!("{}", "Deleted."),
                            Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                        }
                    }
                    "clear" => {
                        match commands::memory::memory_clear(&state, None).await {
                            Ok(count) => println!("Cleared {} entries.", count),
                            Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                        }
                    }
                    _ => eprintln!("{} Unknown memory subcommand: {}. Try: store, search, remember, count, delete, clear", "Usage:".bold(), args[0]),
                }
            }
            "/config" => {
                let args: Vec<&str> = rest.splitn(3, ' ').collect();
                match args.first().copied().unwrap_or("show") {
                    "show" => {
                        let cfg = config::load_provider_config();
                        print_banner(&cfg);
                    }
                    "set" => {
                        if args.len() < 3 {
                            eprintln!("{} Usage: /config set <key> <value>", "Usage:".bold());
                            continue;
                        }
                        let key = args[1];
                        let value = args[2];
                        let mut cli_cfg = config::load_config();
                        let mut provider_cfg = cli_cfg.to_provider_config();
                        match key {
                            "provider" => {
                                provider_cfg.kind = providers::ProviderKind::from_str(value);
                                println!("   provider updated");
                            }
                            "model" => {
                                provider_cfg.model = value.to_string();
                                println!("   model updated");
                            }
                            "base_url" => {
                                provider_cfg.base_url = Some(value.to_string());
                                println!("   base_url updated");
                            }
                            "api_key" => {
                                let _ = config::save_api_key(value);
                                println!("   api_key updated (stored in .env)");
                            }
                            "permission_mode" => {
                                match value {
                                    "off" | "on" | "strict" => {
                                        cli_cfg.permission_mode = Some(value.to_string());
                                        let _ = config::save_config(&cli_cfg);
                                        println!("   permission_mode set to {}", value);
                                    }
                                    _ => eprintln!("{} permission_mode must be off, on, or strict", "Error:".bold()),
                                }
                                continue;
                            }
                            _ => {
                                println!("{} Unknown key: {}", "Error:".bold(), key);
                                continue;
                            }
                        }
                        cli_cfg = config::CliConfig::from_provider_config(&provider_cfg);
                        let _ = config::save_config(&cli_cfg);
                    }
                    "providers" => {
                        for (i, kind) in providers::ProviderKind::all().iter().enumerate() {
                            println!("   {:<4} {}", i + 1, kind);
                        }
                    }
                    _ => eprintln!("{} Usage: /config [show|set|providers]", "Usage:".bold()),
                }
            }
            "/models" => {
                let provider_name = if rest.is_empty() { None } else { Some(rest.clone()) };
                let fut = cmd_models(provider_name, None, None);
                if let Err(e) = fut.await {
                    print_error_detail("Failed to fetch models.", &e.to_string());
                }
            }
            "/provider" => {
                if let Err(e) = cmd_provider().await {
                    print_error_detail("Provider setup failed.", &e.to_string());
                }
            }
            "/cost" => {
                println!("{}", commands::chat::cost_report());
            }
            "/retry" => {
                let content = match &last_message {
                    Some(m) => m.clone(),
                    None => {
                        eprintln!("{} No previous message to retry.", "Error:".bold());
                        continue;
                    }
                };
                let mut provider_cfg = config::load_provider_config();
                if let Err(e) = ensure_model_ready(&mut provider_cfg).await {
                    print_error_detail("Model not ready.", &e.to_string());
                    continue;
                }
                let state = create_state_with_provider(provider_cfg.clone());
                let request = commands::chat::StreamMessageRequest {
                    content: content.clone(),
                    agent_type: "chat".into(),
                    provider: Some(provider_cfg.clone()),
                    system_prompt: system_prompt.clone(),
                    permission_mode: config::load_config().permission_mode.unwrap_or_default(),
                };
                let emitter = TerminalPrinter::new();
                match commands::chat::stream_message(&state, request, &emitter).await {
                    Ok(response) => {
                        history.push(HistoryEntry { role: "user".into(), content });
                        if !response.is_empty() {
                            history.push(HistoryEntry { role: "assistant".into(), content: response });
                        }
                    }
                    Err(e) => print_error_detail("LLM request failed.", &e),
                }
            }
            "/init" => {
                if rest.is_empty() {
                    system_prompt = None;
                    println!("{} System prompt cleared.", "Init:".bold());
                } else {
                    system_prompt = Some(rest.clone());
                    println!("{} System prompt set ({} chars).", "Init:".bold(), rest.len());
                }
            }
            "/history" => {
                let n = rest.parse::<usize>().unwrap_or(history.len());
                let start = history.len().saturating_sub(n.max(1));
                if history.is_empty() {
                    println!("{} No messages yet.", "(empty)".bold());
                } else {
                    for (i, entry) in history[start..].iter().enumerate() {
                        let idx = start + i + 1;
                        let preview: String = entry.content.chars().take(200).collect();
                        if entry.content.len() > 200 {
                            println!("  {}. {}: {}...", idx, entry.role, preview);
                        } else {
                            println!("  {}. {}: {}", idx, entry.role, preview);
                        }
                    }
                }
            }
            "/save" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /save <name>", "Usage:".bold());
                    continue;
                }
                match save_history(&rest, &history) {
                    Ok(()) => println!("{} Session saved as '{}' ({} messages).", "Saved:".bold(), rest, history.len()),
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/load" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /load <name>", "Usage:".bold());
                    continue;
                }
                match load_history(&rest) {
                    Ok(loaded) => {
                        let count = loaded.len();
                        history = loaded;
                        println!("{} Session '{}' loaded ({} messages).", "Loaded:".bold(), rest, count);
                    }
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/sessions" => {
                match list_sessions() {
                    Ok(list) if list.is_empty() => println!("{} No saved sessions.", "(empty)".bold()),
                    Ok(list) => {
                        println!("{}", "Saved sessions:".bold());
                        for s in &list {
                            println!("  {}", s);
                        }
                    }
                    Err(e) => eprintln!("{} {}", "Error:".bold(), e),
                }
            }
            "/help" | "/h" => {
                println!("{}", "Chat:".bold().underline());
                println!("  /chat <msg>     Send a message to the LLM (or just type it)");
                println!("  /retry          Re-send the previous message");
                println!("  /init [prompt]  Set or clear the system prompt");
                println!("  /history [n]    Show last N messages (default all)");
                println!("  /cost           Show token usage for the session");
                println!();
                println!("{}", "Development:".bold().underline());
                println!("  /plan <task>    Generate a structured plan");
                println!("  /plan-status    View the current plan");
                println!("  /plan-approve   Approve the plan for building");
                println!("  /build [auto]   Execute build from approved plan");
                println!("  /review <file>  Run Gate + LLM review on a file");
                println!("  /gate <file>    Run Gate checks only");
                println!("  /memory ...     Memory operations (search, store, remember, ...)");
                println!();
                println!("{}", "Configuration:".bold().underline());
                println!("  /config ...     Show, set, or list providers");
                println!("                   /config set permission_mode <off|on|strict>");
                println!("  /provider       Interactive provider setup");
                println!("  /models         Fetch and list models");
                println!();
                println!("{}", "Session:".bold().underline());
                println!("  /save <name>    Save chat history to a named session");
                println!("  /load <name>    Load a saved session");
                println!("  /sessions       List saved sessions");
                println!();
                println!("{}", "System:".bold().underline());
                println!("  /help           Show this help");
                println!("  /exit           Exit the REPL");
                println!();
                println!("{}", "Tip: Just type text without / to send a chat message.");
            }
            "/exit" | "/quit" | "/q" => {
                println!("{}", "Goodbye.");
                break;
            }
            _ => {
                eprintln!("{} Unknown command: {}. Type /help for available commands.", "Error:".bold(), cmd);
            }
        }
    }

    Ok(())
}
