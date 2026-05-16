# 执行回放

## 概念

每次 Agent 会话完整录制成可回放的脚本（标准格式），支持时间线拖动、分支回溯。

## 录制格式

```json
{
  "session_id": "uuid",
  "created_at": "2026-05-16T...",
  "task": "original user request",
  "frames": [
    {
      "ts_ms": 0,
      "event": "user_input",
      "content": "Run pwd"
    },
    {
      "ts_ms": 1200,
      "event": "tool_call",
      "tool": "bash",
      "input": {"command": "pwd"},
      "output": "/home/dzj/..."
    },
    {
      "ts_ms": 2400,
      "event": "assistant_text",
      "content": "Current directory is..."
    }
  ]
}
```

## 回放模式

```sh
agent_host replay session_2026-05-16.json
# TUI 中按 → 逐帧播放，按 ← 回退，Home/End 跳转
```

## 用途

- 调试 Agent 决策过程
- 训练新 Agent（用成功案例作为 few-shot 示例）
- 回归测试（固化测试用例）
