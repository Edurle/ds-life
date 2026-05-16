# 第3步：实现 Agent 主循环 (Lua)

## 产出文件

| 文件 | 说明 |
|------|------|
| `agent_plugins/system_core/main_loop.lua` | Agent 主循环 |
| `agent_host/src/main.rs`（部分） | value_to_lua / lua_to_value 辅助函数 |

## main_loop.lua 核心逻辑

```
for step = 1, max_steps:
    1. 调用 host.llm_chat(messages) 获取 LLM 响应
    2. 解析 JSON 响应
    3. 若 end_turn → 返回最终文本
    4. 若有 tool_use → 调用对应的 host.xxx 函数
    5. 构造 tool_result 消息追加到 messages
    6. 继续循环
```

### 工具调用格式（修复关键）

每次 `tool_use` 块包含 `name`, `input`, `id`。Lua 侧根据 `name` 选择对应 `host[name]` 函数，用 `pcall` 安全调用，然后构建 `tool_result`：

```lua
{
    type = "tool_result",
    tool_use_id = block.id,
    content = result_text,
    is_error = false,
}
```

### JSON 转换（Rust 侧）

`value_to_lua` 和 `lua_to_value` 实现 JSON ↔ Lua Value 的互转。关键改进：

- table 类型判断：检查所有键是否为整数（非依赖 `#` 操作符）
- 支持嵌套数组和对象

## 验证

```sh
cd agent_host && cargo build
# 应编译成功
```
