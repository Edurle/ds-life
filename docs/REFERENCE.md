# 编译故障排查与参考

## 常见编译错误

### 1. `cannot find macro lazy_static`

```
error: cannot find macro `lazy_static` in this scope
```

**原因**：`Cargo.toml` 缺 `lazy_static` 依赖。

**解决**：在 `[dependencies]` 中添加 `lazy_static = "1.5"`。

### 2. `Text` is not a `Widget`

```
error: the trait `Widget` is not implemented for `Text<'_>`
```

**原因**：原计划使用 `ratatui::text::Text` 尝试渲染，但 `Text` 不是 Widget。

**解决**：使用 `Paragraph::new(...)` 替代。

### 3. lifetime error in async closure

```
error: lifetime may not live long enough
  |  async move {
  |         ^^ ...but this value lives only for the body of this closure
```

**原因**：`lua_to_value(&_lua, messages)` 在 async block 内部调用，`&_lua` 引用跨了 await 点。

**解决**：在同步闭包内完成 `lua_to_value` 转换，只将 `serde_json::Value`（owned）move 进 async block。

### 4. Agent 返回纯文本，不调用工具

**原因**：Anthropic API 不知道有哪些工具可用。

**解决**：确认 `llm.rs` 中的 `TOOLS_DEFINITION` 已正确定义，且随请求体传入。

### 5. `tool_use_id` mismatch

```
Error: tool_use_id ... doesn't correspond to an actual tool_use block
```

**原因**：`tool_result` 的 `tool_use_id` 与 `assistant` 消息中的 `id` 不匹配。

**解决**：确保 Lua 侧从对应的 `tool_use` 块中提取 `id` 字段存入 `tool_use_id`。

### 6. 编译时 `vendored` 失败

```
error: failed to run custom build command for 'liblua-sys v...'
```

**原因**：系统缺少 C 编译器。

**解决**：`sudo apt install build-essential` (Linux) 或 `xcode-select --install` (macOS)。

## 工具定义参考

`TOOLS_DEFINITION` 中每个工具必含字段：
- `name` — 工具名，与 Lua 侧 `host[name]` 对应
- `description` — 描述，指导 LLM 何时调用
- `input_schema` — JSON Schema 定义参数

## API 版本

| 组件 | 版本 |
|------|------|
| mlua | 0.9.x |
| tokio | 1.x (full) |
| reqwest | 0.12.x |
| ratatui | 0.26.x |
| crossterm | 0.27.x |
| lazy_static | 1.5.x |
| Anthropic API | 2023-06-01 |
| 推荐模型 | claude-3-5-haiku-20241022 |
