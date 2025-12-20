//! ADI Agent Loop Plugin
//!
//! Provides MCP tools and resources for autonomous LLM agents.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError, ServiceHandle,
    ServiceMethod, ServiceVTable, ServiceVersion, SERVICE_MCP_RESOURCES, SERVICE_MCP_TOOLS,
};
use once_cell::sync::OnceCell;
use serde_json::{json, Value};
use std::ffi::c_void;
use tokio::runtime::Runtime;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

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
    // Initialize tokio runtime
    let _ = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    });

    unsafe {
        let host = (*ctx).host();

        // Register MCP tools service
        let tools_descriptor = ServiceDescriptor::new(
            SERVICE_MCP_TOOLS,
            ServiceVersion::new(1, 0, 0),
            "adi.agent-loop",
        )
        .with_description("MCP tools for agent operations");

        let tools_handle = ServiceHandle::new(
            SERVICE_MCP_TOOLS,
            ctx as *const c_void,
            &MCP_TOOLS_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(tools_descriptor, tools_handle) {
            host.error(&format!("Failed to register MCP tools service: {}", code));
            return code;
        }

        // Register MCP resources service
        let resources_descriptor = ServiceDescriptor::new(
            SERVICE_MCP_RESOURCES,
            ServiceVersion::new(1, 0, 0),
            "adi.agent-loop",
        )
        .with_description("MCP resources for agent session data");

        let resources_handle = ServiceHandle::new(
            SERVICE_MCP_RESOURCES,
            ctx as *const c_void,
            &MCP_RESOURCES_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(resources_descriptor, resources_handle) {
            host.error(&format!(
                "Failed to register MCP resources service: {}",
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

// === MCP Tools Service ===

static MCP_TOOLS_VTABLE: ServiceVTable = ServiceVTable {
    invoke: mcp_tools_invoke,
    list_methods: mcp_tools_list_methods,
};

extern "C" fn mcp_tools_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    let result = match method.as_str() {
        "list_tools" => Ok(list_tools_json()),
        "call_tool" => {
            let params: Value = match serde_json::from_str(args.as_str()) {
                Ok(v) => v,
                Err(e) => {
                    return RResult::RErr(ServiceError::invocation_error(format!(
                        "Invalid args: {}",
                        e
                    )))
                }
            };

            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = params.get("args").cloned().unwrap_or(json!({}));

            call_tool(tool_name, &tool_args)
        }
        _ => Err(ServiceError::method_not_found(method.as_str())),
    };

    match result {
        Ok(s) => RResult::ROk(RString::from(s)),
        Err(e) => RResult::RErr(e),
    }
}

extern "C" fn mcp_tools_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("list_tools").with_description("List all available tools"),
        ServiceMethod::new("call_tool").with_description("Call a tool by name with arguments"),
    ]
    .into_iter()
    .collect()
}

fn list_tools_json() -> String {
    let tools = json!([
        {
            "name": "agent_run",
            "description": "Run an agent with a given task",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task": { "type": "string", "description": "Task description for the agent" },
                    "max_steps": { "type": "integer", "default": 10, "description": "Maximum number of steps" }
                },
                "required": ["task"]
            }
        },
        {
            "name": "agent_sessions",
            "description": "List agent sessions",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "default": 10 }
                }
            }
        },
        {
            "name": "agent_session_get",
            "description": "Get details of a specific session",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Session ID" }
                },
                "required": ["id"]
            }
        }
    ]);
    serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string())
}

fn call_tool(tool_name: &str, args: &Value) -> Result<String, ServiceError> {
    // Agent operations require more complex setup, returning placeholder for now
    match tool_name {
        "agent_run" => {
            let task = args
                .get("task")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ServiceError::invocation_error("Missing task"))?;

            // Placeholder - actual implementation would run the agent
            Ok(tool_result(&format!(
                "Agent would execute task: {}. Full implementation requires LLM configuration.",
                task
            )))
        }
        "agent_sessions" => {
            // Placeholder - would list sessions from storage
            Ok(tool_result("[]"))
        }
        "agent_session_get" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ServiceError::invocation_error("Missing id"))?;

            // Placeholder - would fetch session details
            Err(ServiceError::invocation_error(format!(
                "Session {} not found",
                id
            )))
        }
        _ => Err(ServiceError::invocation_error(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    }
}

// === MCP Resources Service ===

static MCP_RESOURCES_VTABLE: ServiceVTable = ServiceVTable {
    invoke: mcp_resources_invoke,
    list_methods: mcp_resources_list_methods,
};

extern "C" fn mcp_resources_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    _args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    let result = match method.as_str() {
        "list_resources" => Ok(list_resources_json()),
        "read_resource" => Err(ServiceError::invocation_error("Not implemented")),
        _ => Err(ServiceError::method_not_found(method.as_str())),
    };

    match result {
        Ok(s) => RResult::ROk(RString::from(s)),
        Err(e) => RResult::RErr(e),
    }
}

extern "C" fn mcp_resources_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("list_resources").with_description("List all available resources"),
        ServiceMethod::new("read_resource").with_description("Read a resource by URI"),
    ]
    .into_iter()
    .collect()
}

fn list_resources_json() -> String {
    let resources = json!([
        {
            "uri": "agent://sessions",
            "name": "Agent Sessions",
            "description": "List of all agent sessions",
            "mimeType": "application/json"
        }
    ]);
    serde_json::to_string(&resources).unwrap_or_else(|_| "[]".to_string())
}

fn tool_result(text: &str) -> String {
    let result = json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    });
    serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
}
