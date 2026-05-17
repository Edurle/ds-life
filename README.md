# ds-life

Rust + Lua 构建的自迭代 AI Agent 系统。

## 功能

- **TUI 交互模式** — 自适应布局、滚动翻页、流式输出、token/缓存/上下文状态监控
- **CLI 模式** — 终端直接执行一次性任务
- **Slash 命令** — `/save /history /load /clear /help`，不冲突终端复用器
- **流式 SSE** — Agent 响应逐 token 显示，类似真实对话
- **会话持久化** — SQLite 存储，支持历史会话管理
- **自我进化** — Agent 可在运行中修改自身 Lua 代码
- **崩溃恢复** — 自动检测失败 + git 回滚
- **文件安全** — 文件路径遍历检查 + 自动 `.bak` 备份 + 原子写入

## 快速开始

```sh
# 配置 API 密钥
export DEEPSEEK_API_KEY=sk-xxxx

# 启动 TUI
cd agent_host && cargo run tui

# 或直接看效果（不调 API）
cargo run tui --demo
```

在 TUI 中：

```
          ┌─ Output ────────────────────────┐
          │                                  │
          │  Agent TUI ready.                │
          │  > List the files in this repo   │
          │  Current directory structure...  │
          │                                  │
          ├─ Input ──────────────────────────┤
          │  Type here...                    │
          ├──────────────────────────────────┤
  ⠋ [normal]  12/24  tok i780/o520  cache 92%  ctx 0.1%
          └──────────────────────────────────┘
```

## 运行模式

```sh
cargo run              # CLI 模式，执行默认任务
cargo run tui          # TUI 交互模式
cargo run tui --demo   # TUI 演示模式（不调 API）
cargo run tui "任务"   # TUI 单次任务
```

## 项目结构

```
├── agent_host/         Rust 核心（main.rs, tui.rs, tools/...）
├── agent_plugins/      Lua 插件（main_loop.lua, skills/...）
├── roadmap/            进化路线图
└── docs/               文档
```

## 依赖

Rust 1.70+、Lua 5.4（内置的 mlua 已 vendored）。无需额外安装 Lua 运行时。
