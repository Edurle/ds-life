# 第4步：连接真实 Anthropic API

## 核心修改

### llm.rs — 完整 API 请求体

Rust 侧组装完整请求体，包含：

```json
{
    "model": "claude-3-5-haiku-20241022",
    "max_tokens": 4096,
    "system": "You are a self-iterating coding agent...",
    "tools": [
        {"name": "read_file", "description": "...", "input_schema": {...}},
        {"name": "write_file", ...},
        {"name": "bash", ...},
        {"name": "edit_session_start", ...},
        {"name": "update_plugin", ...},
        {"name": "edit_session_commit", ...},
        {"name": "edit_session_rollback", ...},
        {"name": "list_skills", ...}
    ],
    "messages": [...]
}
```

### main.rs — 异步修复

`create_async_function` 闭包内，**在同步部分完成 Lua → JSON 转换**，只将 owned 数据 move 进 async block。

```rust
// 正确做法
let host_llm = lua.create_async_function(move |lua, messages: mlua::Value| {
    let rust_val = lua_to_value(&lua, messages)?;  // 同步转换
    let api_key = api_key.clone();
    async move {
        tools::llm::llm_chat(&api_key, "claude-3-5-haiku-20241022", rust_val).await
    }
});
```

### 重试逻辑
- 最多 3 次
- `429 Rate Limited` → 指数退避 (2s, 4s, 8s)
- 网络错误 → 指数退避重试

## 验证

```sh
export ANTHROPIC_API_KEY="你的密钥"
cd agent_host && cargo build
```
