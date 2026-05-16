use futures_util::StreamExt;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::Duration;

/// Token 用量统计。
#[derive(Debug, Default, Clone, Copy)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

impl TokenUsage {
    pub fn cache_hit_rate(&self) -> Option<f64> {
        let total = self.cache_read_tokens + self.cache_creation_tokens;
        if total == 0 { return None; }
        Some(self.cache_read_tokens as f64 / total as f64)
    }

    #[allow(dead_code)]
    pub fn accumulate(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_creation_tokens += other.cache_creation_tokens;
    }
}

lazy_static::lazy_static! {
    static ref STREAMING_HOOK: Mutex<Option<Box<dyn Fn(&str) + Send + Sync>>> = Mutex::new(None);
    static ref TOKEN_USAGE: Mutex<TokenUsage> = Mutex::new(TokenUsage::default());
}

pub fn get_token_usage() -> TokenUsage {
    *TOKEN_USAGE.lock().unwrap()
}

fn accumulate_usage(response: &Value) {
    if let Some(usage) = response.get("usage") {
        let mut global = TOKEN_USAGE.lock().unwrap();
        global.input_tokens += usage["input_tokens"].as_u64().unwrap_or(0);
        global.output_tokens += usage["output_tokens"].as_u64().unwrap_or(0);
        global.cache_read_tokens += usage["cache_read_input_tokens"].as_u64().unwrap_or(0);
        global.cache_creation_tokens += usage["cache_creation_input_tokens"].as_u64().unwrap_or(0);
    }
}

/// 设置全局流式回调钩子。
pub fn set_streaming_hook(hook: Box<dyn Fn(&str) + Send + Sync>) {
    *STREAMING_HOOK.lock().unwrap() = Some(hook);
}

/// 清除全局流式回调钩子。
pub fn clear_streaming_hook() {
    *STREAMING_HOOK.lock().unwrap() = None;
}

/// 取出流式回调钩子（用于 async 闭包内部）。
pub fn take_streaming_hook() -> Option<Box<dyn Fn(&str) + Send + Sync>> {
    STREAMING_HOOK.lock().unwrap().take()
}

/// 放回流式回调钩子。
pub fn restore_streaming_hook(hook: Box<dyn Fn(&str) + Send + Sync>) {
    *STREAMING_HOOK.lock().unwrap() = Some(hook);
}

/// 当前累积块状态
#[derive(Default)]
struct BlockState {
    r#type: Option<String>,
    id: Option<String>,
    name: Option<String>,
    text: String,
    input_json: String,
}

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
                    // 解析 usage 并累计
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        accumulate_usage(&json);
                    }
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

/// 流式调用 Anthropic API。逐 token 回调 `on_token`，返回完整 JSON 响应字符串。
///
/// 流式模式使用 `stream: true`，解析 SSE 事件，重构与 `llm_chat` 相同格式的 JSON。
pub async fn llm_chat_streaming(
    api_key: &str,
    model: &str,
    base_url: &str,
    messages: Value,
    on_token: impl Fn(&str),
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let tools: Value = serde_json::from_str(TOOLS_DEFINITION)?;

    let body = json!({
        "model": model,
        "max_tokens": 4096,
        "system": SYSTEM_PROMPT,
        "tools": tools,
        "messages": messages,
        "stream": true,
    });

    let url = format!("{}/v1/messages", base_url);
    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await?;
        anyhow::bail!("API error (HTTP {}): {}", status, text);
    }

    // ── SSE 解析 + JSON 重构 ──
    let mut stream = resp.bytes_stream();
    let mut buffer = Vec::<u8>::new();

    // 重构响应
    let mut content_blocks: Vec<Value> = Vec::new();
    let mut stop_reason: Option<String> = None;

    let mut block = BlockState::default();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.extend_from_slice(&chunk);

        // 按行解析 SSE
        let text = std::str::from_utf8(&buffer)?.to_owned();
        // SSE 事件以 \n\n 分隔。如果 buffer 末尾不完整，保留到下一块。
        while let Some(pos) = text.find("\n\n") {
            let event_str = &text[..pos];
            parse_sse_line(event_str, &mut block, &mut content_blocks, &mut stop_reason, &on_token);
            buffer.drain(..pos + 2);
        }
    }
    // 处理剩余部分
    if !buffer.is_empty() {
        if let Ok(text) = std::str::from_utf8(&buffer) {
            parse_sse_line(text, &mut block, &mut content_blocks, &mut stop_reason, &on_token);
        }
    }

    // 刷新最后一块
    if let Some(block_type) = block.r#type.take() {
        finalize_block(block_type, &mut block, &mut content_blocks);
    }

    // 组装 JSON
    let response = json!({
        "content": content_blocks,
        "stop_reason": stop_reason.unwrap_or_else(|| "end_turn".to_string()),
    });

    Ok(serde_json::to_string(&response)?)
}

/// 解析单条 SSE 数据行（`data: {...}` 或 `event: xxx`）。
fn parse_sse_line(
    line: &str,
    block: &mut BlockState,
    content_blocks: &mut Vec<Value>,
    stop_reason: &mut Option<String>,
    on_token: &impl Fn(&str),
) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }

    // 解析 data: JSON
    let json_str = if let Some(d) = line.strip_prefix("data: ") {
        d.trim()
    } else {
        return; // event: 行跳过，我们直接从 data 推断
    };

    let Ok(data) = serde_json::from_str::<Value>(json_str) else {
        return;
    };

    let event_type = data["type"].as_str().unwrap_or("");

    match event_type {
        "content_block_start" => {
            // 先结束前一块
            if let Some(t) = block.r#type.take() {
                finalize_block(t, block, content_blocks);
            }
            let cb = &data["content_block"];
            block.r#type = cb["type"].as_str().map(|s| s.to_string());
            block.id = cb["id"].as_str().map(|s| s.to_string());
            block.name = cb["name"].as_str().map(|s| s.to_string());
            block.text = cb["text"].as_str().unwrap_or("").to_string();
            // 如果是 tool_use，input 可能是空对象 {}
            if let Some(input) = cb.get("input") {
                block.input_json = serde_json::to_string(input).unwrap_or_default();
            }
        }
        "content_block_delta" => {
            let delta = &data["delta"];
            match delta["type"].as_str() {
                Some("text_delta") => {
                    if let Some(text) = delta["text"].as_str() {
                        block.text.push_str(text);
                        on_token(text);
                    }
                }
                Some("input_json_delta") => {
                    if let Some(pj) = delta["partial_json"].as_str() {
                        block.input_json.push_str(pj);
                    }
                }
                _ => {}
            }
        }
        "content_block_stop" => {
            if let Some(t) = block.r#type.take() {
                finalize_block(t, block, content_blocks);
            }
        }
        "message_delta" => {
            let delta = &data["delta"];
            if let Some(sr) = delta["stop_reason"].as_str() {
                *stop_reason = Some(sr.to_string());
            }
            // 流式输出的 token 用量在 message_delta 中
            if let Some(usage) = data.get("usage") {
                let mut global = TOKEN_USAGE.lock().unwrap();
                global.output_tokens += usage["output_tokens"].as_u64().unwrap_or(0);
            }
        }
        "message_start" => {
            // 流式的 input token 在 message_start 中
            if let Some(msg) = data.get("message") {
                if let Some(usage) = msg.get("usage") {
                    let mut global = TOKEN_USAGE.lock().unwrap();
                    global.input_tokens += usage["input_tokens"].as_u64().unwrap_or(0);
                    global.cache_read_tokens += usage["cache_read_input_tokens"].as_u64().unwrap_or(0);
                    global.cache_creation_tokens += usage["cache_creation_input_tokens"].as_u64().unwrap_or(0);
                }
            }
        }
        "message_stop" | "ping" => {
            // 忽略
        }
        _ => {
            // 未知事件类型，忽略
        }
    }
}

/// 将累积的块状态转为 JSON 对象并加入 content_blocks。
fn finalize_block(
    block_type: String,
    block: &mut BlockState,
    content_blocks: &mut Vec<Value>,
) {
    match block_type.as_str() {
        "text" => {
            content_blocks.push(json!({
                "type": "text",
                "text": block.text,
            }));
        }
        "tool_use" => {
            let input: Value = serde_json::from_str(&block.input_json).unwrap_or(json!({}));
            content_blocks.push(json!({
                "type": "tool_use",
                "id": block.id.clone().unwrap_or_default(),
                "name": block.name.clone().unwrap_or_default(),
                "input": input,
            }));
        }
        _ => {}
    }
    // 重置块状态
    block.text.clear();
    block.input_json.clear();
    block.id = None;
    block.name = None;
}
