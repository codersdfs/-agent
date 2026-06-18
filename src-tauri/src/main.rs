use clap::{Parser, Subcommand};
use colored::Colorize;
use omega_agent_lib::{AppState, default_db_path, commands, pipeline};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

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
            rt.block_on(cmd_list_models(provider))?;
        }
    }

    Ok(())
}

// ─── Clap CLI ─────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "omega", version, about = "Omega Agent CLI — AI coding assistant")]
struct Cli {
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
    /// List available models for a provider
    ListModels {
        /// Provider name (optional, defaults to current)
        provider: Option<String>,
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
    AppState::new(&db_path)
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

// ─── Command handlers ──────────────────────────────────────────────────────────

async fn cmd_chat(message: String) -> anyhow::Result<()> {
    let state = create_state();
    let request = commands::chat::StreamMessageRequest {
        content: message,
        agent_type: "plan".into(),
        provider: None,
        system_prompt: None,
    };
    match commands::chat::stream_message(&state, request).await {
        Ok(content) => {
            println!("\n{}", content);
        }
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
    }
    Ok(())
}

async fn cmd_plan(task: String) -> anyhow::Result<()> {
    let state = create_state();
    match commands::plan_cmd::generate_plan(&state, task).await {
        Ok(payload) => {
            print_plan(&payload.plan);
        }
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
    }
    Ok(())
}

async fn cmd_plan_status() -> anyhow::Result<()> {
    let state = create_state();
    match commands::plan_cmd::get_plan(&state).await {
        Ok(Some(plan)) => print_plan(&plan),
        Ok(None) => println!("{}", "No plan has been generated yet.".yellow()),
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
    }
    Ok(())
}

async fn cmd_plan_approve() -> anyhow::Result<()> {
    let state = create_state();
    match commands::plan_cmd::approve_plan(&state).await {
        Ok(msg) => println!("{}", msg.green()),
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
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
                let icon = if entry.success { "✓".green() } else { "✗".red() };
                println!("  {} step {} ({}): {}ms", icon, entry.step_index, entry.tool, entry.duration_ms);
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
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
            println!("{} {}", "Passed:".bold(), if output.score_breakdown.passed { "Yes".green() } else { "No".red() });
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
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
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
            println!("{} {}", "Passed:".bold(), if result.passed { "Yes".green() } else { "No".red() });
            for v in &result.violations {
                println!("  [{}] {} — {}", v.category.bold(), v.message, v.tool_hint.as_deref().unwrap_or(""));
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
    }
    Ok(())
}

async fn cmd_memory(cmd: MemorySubcommand) -> anyhow::Result<()> {
    let state = create_state();
    match cmd {
        MemorySubcommand::Store { key, value, layer } => {
            let req = commands::memory::MemoryStoreRequest { key, value, layer };
            match commands::memory::memory_store(&state, req).await {
                Ok(msg) => println!("{}", msg.green()),
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }
        MemorySubcommand::Search { query, layer, limit } => {
            let req = commands::memory::MemorySearchRequest { query, layer, limit: Some(limit) };
            match commands::memory::memory_search(&state, req).await {
                Ok(resp) => {
                    if resp.entries.is_empty() {
                        println!("{}", "No results found.".yellow());
                    } else {
                        for (i, entry) in resp.entries.iter().enumerate() {
                            let relevance = resp.relevance.get(i).copied().unwrap_or(0.0);
                            println!("  [{}] {} (relevance: {:.0}%)", entry.id, entry.key, relevance * 100.0);
                            println!("       layer: {:?} | value: {}", entry.layer, entry.value);
                        }
                    }
                }
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }
        MemorySubcommand::Remember { key, layer } => {
            match commands::memory::memory_remember(&state, key, layer).await {
                Ok(Some(value)) => println!("{}", value),
                Ok(None) => println!("{}", "Not found.".yellow()),
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }
        MemorySubcommand::Count { layer } => {
            match commands::memory::memory_count(&state, layer).await {
                Ok(count) => println!("{} entries", count),
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }
        MemorySubcommand::Delete { id } => {
            match commands::memory::memory_delete(&state, id).await {
                Ok(()) => println!("{}", "Deleted.".green()),
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }
        MemorySubcommand::Clear { layer } => {
            match commands::memory::memory_clear(&state, layer).await {
                Ok(count) => println!("Cleared {} entries.", count),
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }
    }
    Ok(())
}

async fn cmd_config(cmd: ConfigSubcommand) -> anyhow::Result<()> {
    let state = create_state();
    match cmd {
        ConfigSubcommand::Show => {
            let config = state.provider_config.lock().unwrap();
            println!("{}", "Current Configuration:".bold());
            println!("  provider:  {}", config.kind);
            println!("  model:     {}", config.model);
            println!("  base_url:  {}", config.base_url.as_deref().unwrap_or("(default)"));
            println!("  max_tokens: {}", config.max_tokens);
            println!("  temperature: {}", config.temperature);
            println!("  api_key:   {}", if config.api_key.as_deref().unwrap_or("").is_empty() { "(not set)" } else { "****" });
        }
        ConfigSubcommand::Set { key, value } => {
            let mut config = state.provider_config.lock().unwrap();
            match key.as_str() {
                "provider" => config.kind = providers::ProviderKind::from_str(&value),
                "model" => config.model = value,
                "base_url" => config.base_url = Some(value),
                "api_key" => config.api_key = Some(value),
                "max_tokens" => config.max_tokens = value.parse().unwrap_or(4096),
                "temperature" => config.temperature = value.parse().unwrap_or(0.7),
                _ => eprintln!("{} Unknown key: {}. Try: provider, model, base_url, api_key, max_tokens, temperature", "Error:".red().bold(), key),
            }
            println!("{} set to {}", key, "updated".green());
        }
        ConfigSubcommand::Providers => {
            println!("{}", "Available providers:".bold());
            for kind in providers::ProviderKind::all() {
                println!("  {}", kind);
            }
        }
    }
    Ok(())
}

async fn cmd_list_models(provider: Option<String>) -> anyhow::Result<()> {
    let state = create_state();
    let config = if let Some(ref name) = provider {
        let kind = providers::ProviderKind::from_str(name);
        let mut c = state.provider_config.lock().unwrap().clone();
        c.kind = kind;
        c
    } else {
        state.provider_config.lock().unwrap().clone()
    };
    match commands::chat::list_models(&config) {
        Ok(models) => {
            for m in models {
                println!("  {}", m);
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
    }
    Ok(())
}

// ─── REPL ──────────────────────────────────────────────────────────────────────

async fn repl() -> anyhow::Result<()> {
    let state = create_state();
    use std::io::{stdin, stdout, Write};

    println!("{}", "Ω Omega Agent — interactive mode. Type /help for commands.".bright_purple().bold());

    loop {
        print!("{} ", "Ω>".bright_purple().bold());
        let _ = stdout().flush();
        let mut line = String::new();
        if stdin().read_line(&mut line).is_err() || line.trim().is_empty() {
            continue;
        }
        let line = line.trim().to_string();

        if !line.starts_with('/') {
            // Plain text — send to chat
            let request = commands::chat::StreamMessageRequest {
                content: line,
                agent_type: "plan".into(),
                provider: None,
                system_prompt: None,
            };
            match commands::chat::stream_message(&state, request).await {
                Ok(_) => {}
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let rest = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

        match cmd.as_str() {
            "/chat" | "/c" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /chat <message>", "Usage:".yellow().bold());
                    continue;
                }
                let request = commands::chat::StreamMessageRequest {
                    content: rest,
                    agent_type: "plan".into(),
                    provider: None,
                    system_prompt: None,
                };
                match commands::chat::stream_message(&state, request).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                }
            }
            "/plan" | "/p" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /plan <task description>", "Usage:".yellow().bold());
                    continue;
                }
                match commands::plan_cmd::generate_plan(&state, rest).await {
                    Ok(payload) => print_plan(&payload.plan),
                    Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                }
            }
            "/plan-status" => {
                match commands::plan_cmd::get_plan(&state).await {
                    Ok(Some(plan)) => print_plan(&plan),
                    Ok(None) => println!("{}", "No plan has been generated yet.".yellow()),
                    Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                }
            }
            "/plan-approve" | "/approve" => {
                match commands::plan_cmd::approve_plan(&state).await {
                    Ok(msg) => println!("{}", msg.green()),
                    Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
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
                    Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                }
                if auto_approve {
                    let _ = commands::build_cmd::set_build_config(&state, false).await;
                }
            }
            "/review" | "/r" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /review <file> [context]", "Usage:".yellow().bold());
                    continue;
                }
                let (file_path, ctx) = match rest.split_once(' ') {
                    Some((f, c)) => (f.to_string(), c.to_string()),
                    None => (rest.clone(), String::new()),
                };
                let code = match std::fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{} Failed to read {}: {}", "Error:".red().bold(), file_path, e);
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
                    Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                }
            }
            "/gate" | "/g" => {
                if rest.is_empty() {
                    eprintln!("{} Usage: /gate <file>", "Usage:".yellow().bold());
                    continue;
                }
                let content = match std::fs::read_to_string(&rest) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{} Failed to read {}: {}", "Error:".red().bold(), rest, e);
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
                    Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                }
            }
            "/memory" | "/m" => {
                let args: Vec<&str> = rest.splitn(4, ' ').collect();
                if args.is_empty() || args[0].is_empty() {
                    eprintln!("{} Usage: /memory <search|store|remember|count|delete|clear> ...", "Usage:".yellow().bold());
                    continue;
                }
                match args[0] {
                    "store" => {
                        if args.len() < 3 {
                            eprintln!("{} Usage: /memory store <key> <value> [layer]", "Usage:".yellow().bold());
                            continue;
                        }
                        let key = args[1];
                        let value = args[2];
                        let layer = args.get(3).copied().unwrap_or("session");
                        let req = commands::memory::MemoryStoreRequest { key: key.into(), value: value.into(), layer: layer.into() };
                        match commands::memory::memory_store(&state, req).await {
                            Ok(msg) => println!("{}", msg.green()),
                            Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                        }
                    }
                    "search" => {
                        if args.len() < 2 {
                            eprintln!("{} Usage: /memory search <query> [layer] [limit]", "Usage:".yellow().bold());
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
                            Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                        }
                    }
                    "remember" => {
                        if args.len() < 2 {
                            eprintln!("{} Usage: /memory remember <key>", "Usage:".yellow().bold());
                            continue;
                        }
                        match commands::memory::memory_remember(&state, args[1].into(), None).await {
                            Ok(Some(value)) => println!("{}", value),
                            Ok(None) => println!("{}", "Not found.".yellow()),
                            Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                        }
                    }
                    "count" => {
                        match commands::memory::memory_count(&state, None).await {
                            Ok(count) => println!("{} entries", count),
                            Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                        }
                    }
                    "delete" => {
                        if args.len() < 2 {
                            eprintln!("{} Usage: /memory delete <id>", "Usage:".yellow().bold());
                            continue;
                        }
                        match commands::memory::memory_delete(&state, args[1].into()).await {
                            Ok(()) => println!("{}", "Deleted.".green()),
                            Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                        }
                    }
                    "clear" => {
                        match commands::memory::memory_clear(&state, None).await {
                            Ok(count) => println!("Cleared {} entries.", count),
                            Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                        }
                    }
                    _ => eprintln!("{} Unknown memory subcommand: {}. Try: store, search, remember, count, delete, clear", "Usage:".yellow().bold(), args[0]),
                }
            }
            "/config" => {
                let args: Vec<&str> = rest.splitn(2, ' ').collect();
                match args.first().copied().unwrap_or("show") {
                    "show" => {
                        let config = state.provider_config.lock().unwrap();
                        println!("{} {}", "provider:".bold(), config.kind);
                        println!("{} {}", "model:".bold(), config.model);
                        println!("{} {}", "base_url:".bold(), config.base_url.as_deref().unwrap_or("(default)"));
                        println!("{} {}", "max_tokens:".bold(), config.max_tokens);
                        println!("{} {}", "temperature:".bold(), config.temperature);
                        println!("{} {}", "api_key:".bold(), if config.api_key.as_deref().unwrap_or("").is_empty() { "(not set)" } else { "****" });
                    }
                    "set" => {
                        if args.len() < 3 {
                            eprintln!("{} Usage: /config set <key> <value>", "Usage:".yellow().bold());
                            continue;
                        }
                        let key = args[1];
                        let value = args[2];
                        let mut config = state.provider_config.lock().unwrap();
                        match key {
                            "provider" => config.kind = providers::ProviderKind::from_str(value),
                            "model" => config.model = value.to_string(),
                            "base_url" => config.base_url = Some(value.to_string()),
                            "api_key" => config.api_key = Some(value.to_string()),
                            "max_tokens" => config.max_tokens = value.parse().unwrap_or(4096),
                            "temperature" => config.temperature = value.parse().unwrap_or(0.7),
                            _ => eprintln!("{} Unknown key: {}", "Error:".red().bold(), key),
                        }
                        println!("{} set to {}", key, "updated".green());
                    }
                    "providers" => {
                        for kind in providers::ProviderKind::all() {
                            println!("  {}", kind);
                        }
                    }
                    _ => eprintln!("{} Usage: /config [show|set|providers]", "Usage:".yellow().bold()),
                }
            }
            "/help" | "/h" => {
                println!("{}", "Commands:".bold().underline());
                println!("  /chat <msg>     Send a message to the LLM (or just type it)");
                println!("  /plan <task>    Generate a structured plan");
                println!("  /plan-status    View the current plan");
                println!("  /plan-approve   Approve the plan for building");
                println!("  /build [auto]   Execute build from approved plan");
                println!("  /review <file>  Run Gate + LLM review on a file");
                println!("  /gate <file>    Run Gate checks only");
                println!("  /memory ...     Memory operations (store, search, remember, count, delete, clear)");
                println!("  /config ...     Configuration (show, set, providers)");
                println!("  /help           Show this help");
                println!("  /exit           Exit the REPL");
                println!();
                println!("{}", "Tip: Just type text without / to send a chat message.".italic());
            }
            "/exit" | "/quit" | "/q" => {
                println!("{}", "Goodbye.".green());
                break;
            }
            _ => {
                eprintln!("{} Unknown command: {}. Type /help for available commands.", "Error:".red().bold(), cmd);
            }
        }
    }

    Ok(())
}
