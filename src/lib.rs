//! ADI Agent Loop Plugin
//!
//! Provides CLI commands for autonomous LLM agents.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError, ServiceHandle,
    ServiceMethod, ServiceVTable, ServiceVersion,
};

/// Plugin-specific CLI service ID
const SERVICE_CLI: &str = "adi.agent-loop.cli";
use serde_json::json;
use std::ffi::c_void;

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.agent-loop",
        "ADI Agent Loop",
        env!("CARGO_PKG_VERSION"),
        "core",
    )
    .with_author("ADI Team")
    .with_description("Autonomous LLM agent with tool execution")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register CLI commands service
        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.agent-loop")
                .with_description("CLI commands for agent operations");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!(
                "Failed to register CLI commands service: {}",
                code
            ));
            return code;
        }

        host.info("ADI Agent Loop plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

// === Plugin Entry Point ===

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RNone,
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}

// === CLI Service VTable ===

static CLI_SERVICE_VTABLE: ServiceVTable = ServiceVTable {
    invoke: cli_invoke,
    list_methods: cli_list_methods,
};

extern "C" fn cli_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    match method.as_str() {
        "run_command" => {
            let result = run_cli_command(args.as_str());
            match result {
                Ok(output) => RResult::ROk(RString::from(output)),
                Err(e) => RResult::RErr(ServiceError::invocation_error(e)),
            }
        }
        "list_commands" => {
            let commands = json!([
                {"name": "run", "description": "Run agent with a task", "usage": "run <task> [--max-iterations <n>] [--yes]"},
                {"name": "config", "description": "Manage configuration", "usage": "config [show|set <key> <value>]"},
                {"name": "tools", "description": "List available tools", "usage": "tools [list]"}
            ]);
            RResult::ROk(RString::from(
                serde_json::to_string(&commands).unwrap_or_default(),
            ))
        }
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

extern "C" fn cli_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("run_command").with_description("Run a CLI command"),
        ServiceMethod::new("list_commands").with_description("List available commands"),
    ]
    .into_iter()
    .collect()
}

fn run_cli_command(context_json: &str) -> Result<String, String> {
    let context: serde_json::Value =
        serde_json::from_str(context_json).map_err(|e| format!("Invalid context: {}", e))?;

    // Parse command and args from context
    let args: Vec<String> = context
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("");
    let cmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    // Parse options from remaining args (--key value format)
    let mut options = serde_json::Map::new();
    let mut i = 0;
    while i < cmd_args.len() {
        if cmd_args[i].starts_with("--") {
            let key = cmd_args[i].trim_start_matches("--");
            if i + 1 < cmd_args.len() && !cmd_args[i + 1].starts_with("--") {
                options.insert(key.to_string(), json!(cmd_args[i + 1]));
                i += 2;
            } else {
                options.insert(key.to_string(), json!(true));
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    // Get positional args (non-option args after subcommand)
    let positional: Vec<&str> = cmd_args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .copied()
        .collect();

    let options_value = serde_json::Value::Object(options);

    match subcommand {
        "run" => cmd_run(&positional, &options_value),
        "config" => cmd_config(&positional),
        "tools" => cmd_tools(&positional),
        "" => {
            let help = "ADI Agent Loop - Autonomous LLM agent with tool execution\n\n\
                        Commands:\n  \
                        run      Run agent with a task\n  \
                        config   Manage configuration\n  \
                        tools    List available tools\n\n\
                        Usage: adi run adi.agent-loop <command> [args]";
            Ok(help.to_string())
        }
        _ => Err(format!("Unknown command: {}", subcommand)),
    }
}

// === Command Implementations ===

fn cmd_run(args: &[&str], options: &serde_json::Value) -> Result<String, String> {
    if args.is_empty() {
        return Err("Missing task. Usage: run <task> [--max-iterations <n>] [--yes]".to_string());
    }

    let task = args[0];
    let max_iterations = options
        .get("max-iterations")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(50u64);
    let auto_approve = options
        .get("yes")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // For now, return a message indicating the agent would run
    // Full implementation requires LLM provider configuration
    let mut output = String::new();
    output.push_str(&format!("Agent Task: {}\n", task));
    output.push_str(&format!("Max Iterations: {}\n", max_iterations));
    output.push_str(&format!("Auto-approve: {}\n\n", auto_approve));
    output.push_str("Note: Full agent execution requires LLM provider configuration.\n");
    output.push_str("Configure your LLM provider in ~/.config/adi/agent.toml");

    Ok(output)
}

fn cmd_config(args: &[&str]) -> Result<String, String> {
    let subcommand = args.first().copied().unwrap_or("show");

    match subcommand {
        "show" => {
            let mut output = String::from("Current configuration:\n\n");
            output.push_str("  model: claude-sonnet-4-20250514\n");
            output.push_str("  max_iterations: 50\n");
            output.push_str("  max_tokens: 100000\n");
            output.push_str("  timeout_ms: 120000\n");
            Ok(output.trim_end().to_string())
        }
        "set" => {
            if args.len() < 3 {
                return Err("Usage: config set <key> <value>".to_string());
            }
            let key = args[1];
            let value = args[2];
            Ok(format!("Set {} = {}", key, value))
        }
        _ => Err(format!(
            "Unknown config subcommand: {}. Use 'show' or 'set'",
            subcommand
        )),
    }
}

fn cmd_tools(args: &[&str]) -> Result<String, String> {
    let subcommand = args.first().copied().unwrap_or("list");

    match subcommand {
        "list" => {
            let mut output = String::from("Available tools:\n\n");
            output.push_str("  (No tools registered - add tools via configuration)\n\n");
            output.push_str("To add tools, edit ~/.config/adi/agent.toml:\n\n");
            output.push_str("  [[tools]]\n");
            output.push_str("  name = \"my_tool\"\n");
            output.push_str("  command = \"my-command\"\n");
            Ok(output.trim_end().to_string())
        }
        _ => Err(format!(
            "Unknown tools subcommand: {}. Use 'list'",
            subcommand
        )),
    }
}
