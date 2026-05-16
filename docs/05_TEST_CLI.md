# 第5步：命令行测试 Agent 循环

## 验证前提

- `agent_host/src/main.rs` 已包含完整 `#[tokio::main]` 异步 main 函数
- `agent_plugins/system_core/main_loop.lua` 已就绪
- `ANTHROPIC_API_KEY` 环境变量已设置

## 运行

```sh
cd /home/dzj/file/ds-life/agent_host
cargo run
```

## 预期行为

Agent 应：
1. 接收任务 `"What is the current working directory? Use bash to run pwd."`
2. 调用 LLM → 返回 `tool_use`（调用 `bash`）
3. Lua 侧调用 `host.bash("pwd")`
4. 返回结果给 LLM
5. LLM 分析 → 输出最终回答

终端打印：
```
Agent response:
The current working directory is /home/dzj/file/ds-life/agent_host
```

如果看到工具被实际调用并返回结果，说明工具调用链路已打通。

## 常见排查

| 症状 | 原因 | 解决 |
|------|------|------|
| Agent 返回纯文本不调用工具 | tools 定义缺失 | 检查 llm.rs 中 TOOLS_DEFINITION 是否正确 |
| JSON parse error | API 响应格式不对 | 检查 llm.rs 请求体 |
| 403/401 | API Key 无效 | `echo $ANTHROPIC_API_KEY` 确认已设置 |
| Agent 卡住不动 | 网络/API 超时 | 检查网络连接 |
