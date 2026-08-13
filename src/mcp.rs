use std::collections::BTreeSet;
use std::io::{self, BufRead, Write};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::cli;
use crate::config::Config;
use crate::core::{AddOutcome, CompleteOutcome, Store};
use crate::todo::Task;

pub fn run() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.context("reading MCP input")?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str(&line) {
            Ok(request) => handle_request(request),
            Err(error) => Some(jsonrpc_error(
                Value::Null,
                -32700,
                format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut stdout, &response).context("encoding MCP response")?;
            writeln!(stdout).context("writing MCP response")?;
            stdout.flush().context("flushing MCP response")?;
        }
    }
    Ok(())
}

fn handle_request(request: Value) -> Option<Value> {
    let Some(request) = request.as_object() else {
        return Some(jsonrpc_error(
            Value::Null,
            -32600,
            "invalid request".to_string(),
        ));
    };
    if request.get("jsonrpc") != Some(&Value::String("2.0".to_string())) {
        return Some(jsonrpc_error(
            Value::Null,
            -32600,
            "invalid request".to_string(),
        ));
    }
    let id = request.get("id").cloned();
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Some(jsonrpc_error(
            Value::Null,
            -32600,
            "invalid request".to_string(),
        ));
    };
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let response = match method {
        "initialize" => json!({
            "protocolVersion": "2025-11-25",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "prumo", "version": env!("CARGO_PKG_VERSION") }
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => call_tool(&params),
        "notifications/initialized" => return None,
        _ => return id.map(|id| jsonrpc_error(id, -32601, format!("method not found: {method}"))),
    };
    id.map(|id| json!({ "jsonrpc": "2.0", "id": id, "result": response }))
}

fn jsonrpc_error(id: Value, code: i32, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_projects",
            "description": "List persistent Prumo projects with open and completed task counts.",
            "inputSchema": { "type": "object", "additionalProperties": false },
            "annotations": { "readOnlyHint": true }
        }),
        json!({
            "name": "list_tasks",
            "description": "List Prumo tasks, optionally for one project and status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "Project name without the + prefix." },
                    "status": { "type": "string", "enum": ["open", "completed", "all"], "default": "open" }
                },
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": true }
        }),
        json!({
            "name": "add_task",
            "description": "Create one task in Prumo. Include +project in the text when applicable.",
            "inputSchema": {
                "type": "object",
                "properties": { "text": { "type": "string", "minLength": 1 } },
                "required": ["text"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false }
        }),
        json!({
            "name": "complete_task",
            "description": "Complete one open task by its current list number. It remains in todo.txt until archived in Prumo.",
            "inputSchema": {
                "type": "object",
                "properties": { "number": { "type": "integer", "minimum": 1 } },
                "required": ["number"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": true }
        }),
    ]
}

fn call_tool(params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return tool_error("missing tool name");
    };
    let args = params.get("arguments").unwrap_or(&Value::Null);
    let result = match name {
        "list_projects" => load_store(false).map(|store| project_data(&store, &Config::load())),
        "list_tasks" => load_store(false).and_then(|store| task_data(&store, args)),
        "add_task" => load_store(true).and_then(|mut store| add_task(&mut store, args)),
        "complete_task" => load_store(false).and_then(|mut store| complete_task(&mut store, args)),
        _ => Err(format!("unknown tool: {name}")),
    };
    match result {
        Ok(value) => tool_result(value),
        Err(error) => tool_error(error),
    }
}

fn tool_result(value: Value) -> Value {
    json!({ "content": [{ "type": "text", "text": value.to_string() }] })
}

fn tool_error(error: impl ToString) -> Value {
    json!({ "content": [{ "type": "text", "text": error.to_string() }], "isError": true })
}

fn load_store(create_if_missing: bool) -> Result<Store, String> {
    let path = if create_if_missing {
        cli::resolve_path(None)
    } else {
        cli::resolve_read_path()
    }
    .map_err(|error| format!("resolving todo file: {error}"))?;
    let body = std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    let done = cli::done_path(&path);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    Ok(Store::open_sync_with_done(path, done, body, today))
}

fn project_data(store: &Store, config: &Config) -> Value {
    let mut names: BTreeSet<String> = config.project_known.iter().cloned().collect();
    names.extend(config.project_archived.iter().cloned());
    names.extend(
        store
            .tasks()
            .iter()
            .flat_map(|task| task.projects.iter().cloned()),
    );
    names.extend(
        store
            .archive()
            .tasks()
            .iter()
            .flat_map(|task| task.projects.iter().cloned()),
    );
    let projects: Vec<Value> = names
        .into_iter()
        .map(|name| {
            let open = store
                .tasks()
                .iter()
                .filter(|task| !task.done && task.projects.iter().any(|project| project == &name))
                .count();
            let completed = store
                .archive()
                .tasks()
                .iter()
                .chain(store.tasks().iter().filter(|task| task.done))
                .filter(|task| task.projects.iter().any(|project| project == &name))
                .count();
            json!({
                "name": name,
                "archived": config.project_archived.contains(&name),
                "open_count": open,
                "completed_count": completed
            })
        })
        .collect();
    json!({ "projects": projects })
}

fn task_data(store: &Store, args: &Value) -> Result<Value, String> {
    let project = args.get("project").and_then(Value::as_str);
    let status = args.get("status").and_then(Value::as_str).unwrap_or("open");
    if !matches!(status, "open" | "completed" | "all") {
        return Err("status must be open, completed, or all".to_string());
    }
    let include_open = matches!(status, "open" | "all");
    let include_done = matches!(status, "completed" | "all");
    let mut tasks = Vec::new();
    if include_open {
        tasks.extend(
            store
                .tasks()
                .iter()
                .enumerate()
                .filter(|(_, task)| !task.done && matches_project(task, project))
                .map(|(index, task)| task_value(index + 1, "todo", task)),
        );
    }
    if include_done {
        tasks.extend(
            store
                .tasks()
                .iter()
                .enumerate()
                .filter(|(_, task)| task.done && matches_project(task, project))
                .map(|(index, task)| task_value(index + 1, "todo", task)),
        );
        tasks.extend(
            store
                .archive()
                .tasks()
                .iter()
                .enumerate()
                .filter(|(_, task)| matches_project(task, project))
                .map(|(index, task)| task_value(index + 1, "done", task)),
        );
    }
    Ok(json!({ "tasks": tasks }))
}

fn matches_project(task: &Task, project: Option<&str>) -> bool {
    project.is_none_or(|name| {
        task.projects
            .iter()
            .any(|task_project| task_project == name)
    })
}

fn task_value(number: usize, source: &str, task: &Task) -> Value {
    json!({
        "number": number,
        "source": source,
        "raw": task.raw,
        "completed": task.done,
        "completed_at": task.done_date,
        "projects": task.projects,
        "contexts": task.contexts,
        "due": task.due
    })
}

fn add_task(store: &mut Store, args: &Value) -> Result<Value, String> {
    let text = args
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "text is required".to_string())?;
    match store.add_line(text) {
        AddOutcome::Added { abs } => {
            Ok(json!({ "task": task_value(abs + 1, "todo", &store.tasks()[abs]) }))
        }
        AddOutcome::Empty => Err("text is required".to_string()),
        AddOutcome::Aborted(_) => Err("todo file changed on disk; retry".to_string()),
        AddOutcome::Error(error) => Err(error.to_string()),
    }
}

fn complete_task(store: &mut Store, args: &Value) -> Result<Value, String> {
    let number = args
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| "number must be a positive integer".to_string())? as usize;
    let abs = number - 1;
    let Some(task) = store.tasks().get(abs) else {
        return Err(format!("no open task {number}"));
    };
    if task.done {
        return Err("task is already completed".to_string());
    }
    let completed = match store.toggle_complete(abs) {
        CompleteOutcome::Completed { abs } | CompleteOutcome::CompletedSpawned { abs, .. } => {
            store.tasks()[abs].clone()
        }
        CompleteOutcome::Uncompleted { .. } => return Err("task is already completed".to_string()),
        CompleteOutcome::OutOfRange => return Err(format!("no open task {number}")),
        CompleteOutcome::Aborted(_) => return Err("todo file changed on disk; retry".to_string()),
        CompleteOutcome::Error(error) => return Err(error.to_string()),
    };
    Ok(json!({ "task": task_value(number, "todo", &completed) }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn tools_have_stable_names() {
        let tool_defs = tools();
        let names: Vec<&str> = tool_defs
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect();
        assert_eq!(
            names,
            ["list_projects", "list_tasks", "add_task", "complete_task"]
        );
    }

    #[test]
    fn task_json_marks_its_source() {
        let task = crate::todo::parse_line("pay invoice +client").unwrap();
        let value = task_value(2, "todo", &task);
        assert_eq!(value["number"], 2);
        assert_eq!(value["source"], "todo");
        assert_eq!(value["projects"], json!(["client"]));
    }

    #[test]
    fn invalid_request_gets_a_jsonrpc_error() {
        let response = handle_request(json!({ "jsonrpc": "2.0", "id": 1 })).unwrap();
        assert_eq!(response["error"]["code"], -32600);
        assert_eq!(response["id"], Value::Null);
    }
}
