# Agent 写 Agent

## 概念

当前 `self_improve` 只能改自身 Lua 代码。升级为：Agent 能**生成全新的 Agent**（新 Lua 模块 + System Prompt），打包成技能发布。

## 工作流

```
用户: "创建一个专门做正则表达式测试的 Agent"
  ↓
Agent 自省 self_improve 流程
  ↓
生成: agent_plugins/skills/regex_tester/
  ├── SKILL.md           ← system prompt（"你是正则专家..."）
  ├── agent.lua          ← 专用循环逻辑
  └── tools.lua          ← 专用工具（test_regex 等）
  ↓
注册到技能市场，可被其他 Agent 加载
```

## 影响

- Agent 成为 Agent 工厂
- 技能市场的供给端由 Agent 自主生产
- 可能出现"Agent 谱系"（Agent A → 生成 Agent B → B 继续生成...）
