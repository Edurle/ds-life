# 流式输出（Server-Sent Events）

> 当前状态：非流式，用户等待完整响应才看到结果。
> 目标：逐 token 实时推送，TUI 中看到 Agent 逐字"思考"。

## Anthropic SSE 格式

Anthropic Messages API 支持流式（添加 `"stream": true` 到请求体）。响应格式：

```
event: content_block_start
data: {"type":"content_block_start","content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"The"}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":" current"}}

...

event: message_stop
data: {"type":"message_stop"}
```

注意：流式模式下 `tool_use` 也是逐块传输的。

## 方案设计

### Rust 侧改动

**`src/tools/llm.rs`** — 新增 `llm_chat_streaming` 函数：

```rust
use futures_util::StreamExt;

pub async fn llm_chat_streaming(
    api_key: &str,
    model: &str,
    base_url: &str,
    messages: Value,
    mut on_token: impl FnMut(&str),
) -> anyhow::Result<String> {
    let body = json!({
        "model": model,
        "max_tokens": 4096,
        "system": SYSTEM_PROMPT,
        "tools": tools,
        "messages": messages,
        "stream": true,           // ← 关键
    });

    let resp = client
        .post(format!("{}/v1/messages", base_url))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await?;

    let mut stream = resp.bytes_stream();
    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = std::str::from_utf8(&chunk)?;
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(event) = serde_json::from_str::<Value>(data) {
                    if let Some(delta) = event["delta"]["text"].as_str() {
                        full_text.push_str(delta);
                        on_token(delta);
                    }
                }
            }
        }
    }
    Ok(full_text)
}
```

### Lua 侧改动

**`main_loop.lua`** — `run_agent` 接收 callback：

```lua
function run_agent(task, max_steps, on_token)
    -- 在循环中调用 host.llm_chat_streaming(messages, on_token)
    -- on_token 是 Lua 函数，每收到 token 调用
end
```

### TUI 侧改动

**`src/tui.rs`** — 每收到 token 立即追加到输出面板并刷新：

```rust
crate::run_agent_once_streaming(&task, |token| {
    output_text.push_str(token);
    let _ = terminal.draw(|f| { /* 渲染 */ });
}).await;
```

### 依赖

- `futures-util` 已通过 `reqwest` 间接引入，无需额外添加

### 实现步骤

1. `llm.rs` 新增 `llm_chat_streaming`
2. `main.rs` 新增 `run_agent_once_streaming`
3. `tui.rs` 集成流式渲染
4. 编译 + TUI 测试

预估工作量：**1 天**
