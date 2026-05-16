use serde_json::{json, Value};
use std::time::Duration;

/// 固定工具定义（传给 Anthropic API），与 Lua 侧 `host.*` 函数一一对应。
const TOOLS_DEFINITION: &str = r#"[
    {
        "name": "read_file",
        "description": "Read content from a file in the workspace. Use limit to restrict lines returned.",
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Relative file path"},
                "limit": {"type": "integer", "description": "Max lines to return (optional)"}
            },
            "required": ["path"]
        }
    },
    {
        "name": "write_file",
        "description": "Write content to a file in the workspace. Creates parent directories if needed.",
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Relative file path"},
                "content": {"type": "string", "description": "File content to write"}
            },
            "required": ["path", "content"]
        }
    },
    {
        "name": "bash",
        "description": "Run a shell command in the workspace. Use for file operations, builds, and system queries.",
        "input_schema": {
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "Shell command to execute"}
            },
            "required": ["command"]
        }
    },
    {
        "name": "edit_session_start",
        "description": "Start a new edit session for self-modification. Returns a session ID.",
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "update_plugin",
        "description": "Update a plugin file within an edit session. Only files under system_core/ are allowed.",
        "input_schema": {
            "type": "object",
            "properties": {
                "session_id": {"type": "string", "description": "Edit session ID"},
                "file_name": {"type": "string", "description": "File path relative to agent_plugins/ (must start with system_core/)"},
                "content": {"type": "string", "description": "New file content"}
            },
            "required": ["session_id", "file_name", "content"]
        }
    },
    {
        "name": "edit_session_commit",
        "description": "Commit changes in an edit session via git.",
        "input_schema": {
            "type": "object",
            "properties": {
                "session_id": {"type": "string", "description": "Edit session ID"},
                "message": {"type": "string", "description": "Commit message"}
            },
            "required": ["session_id", "message"]
        }
    },
    {
        "name": "edit_session_rollback",
        "description": "Rollback all changes in an edit session.",
        "input_schema": {
            "type": "object",
            "properties": {
                "session_id": {"type": "string", "description": "Edit session ID"}
            },
            "required": ["session_id"]
        }
    },
    {
        "name": "list_skills",
        "description": "List available skills from agent_plugins/skills/ directory.",
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    }
]"#;

/// 固定 System Prompt，指导 Agent 的行为。
const SYSTEM_PROMPT: &str = r#"
You are a self-iterating coding agent running inside a Rust host with Lua scripting.

Capabilities:
- read_file / write_file: access workspace files
- bash: execute shell commands
- edit_session_start / update_plugin / edit_session_commit / edit_session_rollback: self-modify your own Lua code
- list_skills: discover available skills

Rules:
1. Act directly — don't explain what you'll do, just do it.
2. When you have a tool result, analyze it and decide the next action.
3. You may improve your own code (system_core/main_loop.lua) via the edit session tools.
4. Before modifying yourself, start an edit session, make changes, and commit.
5. Keep responses concise and actionable.
"#;

/// 调用 Anthropic Messages API，带完整 system prompt + tools 定义 + 基础重试。
pub async fn llm_chat(
    api_key: &str,
    model: &str,
    base_url: &str,
    messages: Value,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let tools: Value = serde_json::from_str(TOOLS_DEFINITION)?;

    let body = json!({
        "model": model,
        "max_tokens": 4096,
        "system": SYSTEM_PROMPT,
        "tools": tools,
        "messages": messages,
    });

    let max_retries = 3;
    let mut last_error = String::new();

    let url = format!("{}/v1/messages", base_url);

    for attempt in 0..max_retries {
        let resp = client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(r) => {
                let status = r.status();
                let text = r.text().await?;

                if status.is_success() {
                    return Ok(text);
                }

                if status.as_u16() == 429 {
                    // 速率限制：指数退避
                    let delay = Duration::from_secs(2u64.pow(attempt as u32));
                    eprintln!(
                        "Rate limited (attempt {}), retrying in {}s...",
                        attempt + 1,
                        delay.as_secs()
                    );
                    tokio::time::sleep(delay).await;
                    last_error = format!("HTTP {}: {}", status, text);
                    continue;
                }

                anyhow::bail!("API error (HTTP {}): {}", status, text);
            }
            Err(e) => {
                last_error = e.to_string();
                if attempt < max_retries - 1 {
                    let delay = Duration::from_secs(2u64.pow(attempt as u32));
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
        }
    }

    anyhow::bail!(
        "LLM call failed after {} retries: {}",
        max_retries,
        last_error
    )
}
