//! Ensure a kiro agent definition has the duir MCP server configured.
//!
//! On `:kiro --agent=foo`, we find `foo` in `~/.kiro/agents/` or `.kiro/agents/`
//! (by filename or "name" field), create `duir-foo.json` in `.kiro/agents/`
//! with the duir MCP server patched in.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Map, Value};

/// Ensure a patched agent file exists at `.kiro/agents/duir-<name>.json`.
/// Returns the patched agent name to pass to `--agent=`.
///
/// # Errors
///
/// Returns error if agent not found or file I/O fails.
pub fn ensure_agent_patched(root: &Path, agent_name: &str, socket_path: &Path) -> Result<String, String> {
    if agent_name == "duir" {
        write_duir_agent(root, socket_path);
        return Ok("duir".into());
    }

    let home = env::var("HOME").unwrap_or_default();
    let home_dir = Path::new(&home).join(".kiro/agents");
    let source_path = find_agent_by_name(&home_dir, agent_name);
    let local_source = find_agent_by_name(&root.join(".kiro/agents"), agent_name);

    if local_source.is_none() && source_path.is_none() {
        return Err(format!(
            "agent '{agent_name}' not found in ~/.kiro/agents/ or .kiro/agents/"
        ));
    }

    let patched_name = format!("duir-{agent_name}");
    let patched_path = root.join(format!(".kiro/agents/{patched_name}.json"));
    let best_source = source_path.as_deref().or(local_source.as_deref());

    if needs_patch(&patched_path, best_source) {
        let base = load_source(best_source, &patched_path, agent_name)?;
        let patched = patch_agent(base, &patched_name, socket_path)?;
        write_patched(root, &patched_path, &patched)?;
    }
    Ok(patched_name)
}

fn find_agent_by_name(dir: &Path, agent_name: &str) -> Option<PathBuf> {
    let exact = dir.join(format!("{agent_name}.json"));
    if exact.is_file() {
        return Some(exact);
    }
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if agent_json_has_name(&path, agent_name) {
            return Some(path);
        }
    }
    None
}

fn agent_json_has_name(path: &Path, agent_name: &str) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(val) = serde_json::from_str::<Value>(&content) else {
        return false;
    };
    val.get("name").and_then(|n| n.as_str()) == Some(agent_name)
}

fn needs_patch(local: &Path, source: Option<&Path>) -> bool {
    if !local.is_file() {
        return source.is_some();
    }
    if !local_has_duir_mcp(local) {
        return true;
    }
    let Some(source) = source else { return false };
    let Ok(local_mt) = mtime(local) else { return true };
    let Ok(source_mt) = mtime(source) else { return false };
    source_mt > local_mt
}

fn local_has_duir_mcp(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(val) = serde_json::from_str::<Value>(&content) else {
        return false;
    };
    val.get("mcpServers").and_then(|s| s.get("duir")).is_some()
}

fn mtime(path: &Path) -> Result<SystemTime, std::io::Error> {
    fs::metadata(path)?.modified()
}

fn load_source(source: Option<&Path>, local: &Path, agent_name: &str) -> Result<Value, String> {
    if let Some(path) = source {
        let content = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        return serde_json::from_str(&content).map_err(|e| format!("parse {}: {e}", path.display()));
    }
    if local.is_file() {
        let content = fs::read_to_string(local).map_err(|e| format!("read {}: {e}", local.display()))?;
        return serde_json::from_str(&content).map_err(|e| format!("parse {}: {e}", local.display()));
    }
    Ok(serde_json::json!({"name": agent_name, "tools": ["*"]}))
}

fn patch_agent(mut val: Value, patched_name: &str, socket_path: &Path) -> Result<Value, String> {
    let obj = val.as_object_mut().ok_or("agent JSON is not an object")?;
    obj.insert("name".into(), Value::String(patched_name.into()));

    let servers = obj.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let servers_obj = servers.as_object_mut().ok_or("mcpServers not object")?;
    servers_obj.insert("duir".into(), duir_mcp_server_def(socket_path));

    let allowed = obj.entry("allowedTools").or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = allowed.as_array_mut() {
        let tag = Value::String("@duir".into());
        if !arr.contains(&tag) {
            arr.push(tag);
        }
    }

    let tools = obj.entry("tools").or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = tools.as_array_mut() {
        let tag = Value::String("@duir".into());
        if !arr.contains(&tag) {
            arr.push(tag);
        }
    }

    obj.insert("includeMcpJson".into(), Value::Bool(true));
    Ok(val)
}

fn duir_mcp_server_def(socket_path: &Path) -> Value {
    let bin = env::current_exe().map_or_else(|_| "duir".into(), |p| p.to_string_lossy().into_owned());
    serde_json::json!({
        "command": bin,
        "args": ["--mcp-connect"],
        "env": {"DUIR_MCP_SOCKET": socket_path.to_string_lossy()}
    })
}

fn write_patched(root: &Path, local: &Path, val: &Value) -> Result<(), String> {
    let dir = root.join(".kiro/agents");
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let json = serde_json::to_string_pretty(val).map_err(|e| format!("serialize: {e}"))?;
    fs::write(local, json).map_err(|e| format!("write {}: {e}", local.display()))
}

/// Write `.kiro/agents/duir.json` — the default duir agent with MCP server.
fn write_duir_agent(root: &Path, socket_path: &Path) {
    let agents_dir = root.join(".kiro/agents");
    if fs::create_dir_all(&agents_dir).is_err() {
        return;
    }
    let config = serde_json::json!({
        "name": "duir",
        "mcpServers": {
            "duir": duir_mcp_server_def(socket_path)
        },
        "includeMcpJson": true,
        "tools": ["*"],
        "allowedTools": ["*"],
        "prompt": "You have access to the duir task tree via MCP tools. Use get_context to understand the tree first."
    });
    let json = serde_json::to_string_pretty(&config).unwrap_or_default();
    if let Err(e) = fs::write(agents_dir.join("duir.json"), json) {
        log::error!("agent: write duir.json: {e}");
    }
}

/// Build the full kiro command string, patching agent if needed.
#[must_use]
pub fn build_kiro_cmd(base_cmd: &str, arg: &str, root: &Path, socket_path: &Path) -> String {
    if arg.is_empty() {
        return base_cmd.to_owned();
    }
    let mut parts: Vec<&str> = arg.split_whitespace().collect();
    let mut patched_agent = None;
    for part in &mut parts {
        if let Some(name) = part.strip_prefix("--agent=") {
            match ensure_agent_patched(root, name, socket_path) {
                Ok(patched_name) => patched_agent = Some(patched_name),
                Err(e) => log::error!("agent patch: {e}"),
            }
        }
    }
    patched_agent.as_ref().map_or_else(
        || format!("{base_cmd} {arg}"),
        |name| {
            let fixed_args: Vec<String> = parts
                .iter()
                .map(|p| {
                    if p.starts_with("--agent=") {
                        format!("--agent={name}")
                    } else {
                        (*p).to_owned()
                    }
                })
                .collect();
            format!("{base_cmd} {}", fixed_args.join(" "))
        },
    )
}
