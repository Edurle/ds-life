# 第10步：完善与最终组装

## 产出文件

| 文件 | 说明 |
|------|------|
| `agent_plugins/system_core/sub_agent.lua` | 子智能体机制 |

## 子智能体

子智能体在独立的消息上下文中运行，解决子任务：

```lua
-- 主循环中可调用：
local sub = require("system_core/sub_agent")
local result = sub.spawn("Read the file and summarize", 5)
```

子智能体与主循环共享同一组 `host.*` 工具，但有独立的 `messages` 上下文。

## 配置文件

`agent_plugins/config/` 目录预留，可在后续版本中添加：
- `system_prompt.txt` — 自定义 System Prompt
- `settings.json` — Agent 配置

## 最终组装清单

1. 所有文件到位（见下方）
2. Git 已初始化并提交
3. `ANTHROPIC_API_KEY` 已设置
4. `cargo build` 编译通过
5. 首次测试任务运行

### 文件清单

```
agent_host/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── tools/
    │   ├── mod.rs
    │   ├── fs.rs
    │   ├── bash.rs
    │   ├── llm.rs
    │   └── self_update.rs
    ├── recovery.rs
    └── tui.rs

agent_plugins/
├── system_core/
│   ├── main_loop.lua
│   ├── self_improve.lua
│   ├── skill_loader.lua
│   └── sub_agent.lua
├── skills/example/SKILL.md
├── config/ (empty)
└── memory/ (empty)

scripts/test.lua
docs/*.md
```
