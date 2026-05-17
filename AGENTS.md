# ds-life — 自迭代 AI Agent 系统

ds-life 是一个用 Rust + Lua 构建的自迭代 AI Agent。它通过 Anthropic 兼容 API（默认 DeepSeek V4）驱动，支持 CLI 和 TUI 两种交互模式，能够在运行中修改自己的 Lua 代码实现自我进化。

## 项目结构

```
agent_host/                   ← Rust 核心
├── src/
│   ├── main.rs               ─ 入口 + Lua 环境创建 + Agent 循环
│   ├── tui.rs                ─ TUI 交互界面
│   ├── config.rs             ─ API Key / Model / 配置加载
│   ├── persistence.rs        ─ SQLite 对话持久化
│   ├── recovery.rs           ─ 崩溃检测 + git 回滚
│   └── tools/
│       ├── mod.rs
│       ├── llm.rs            ─ API 调用（非流式 + 流式 SSE）
│       ├── fs.rs             ─ 文件读写工具
│       ├── bash.rs           ─ Shell 执行工具
│       └── self_update.rs    ─ 自我更新工具
│
agent_plugins/                ← Lua 插件
├── system_core/
│   ├── main_loop.lua         ─ Agent 主循环（tool_use 调度）
│   ├── self_improve.lua      ─ 自我改进流程
│   ├── skill_loader.lua      ─ 技能加载器
│   └── sub_agent.lua         ─ 子 Agent 管理
└── skills/                   ← 可安装技能包

roadmap/                      ← 进化路线图
```

## 构建 & 运行

```sh
# 构建
cd agent_host && cargo build

# CLI 模式（默认任务）
cargo run

# TUI 交互模式
cargo run tui

# TUI 演示模式（不调 API）
cargo run tui --demo

# TUI 单次任务
cargo run tui "Read Cargo.toml and summarize"
```

## 配置

| 环境变量 | 用途 | 默认值 |
|----------|------|--------|
| `DEEPSEEK_API_KEY` | API 密钥 | 必填 |
| `DEEPSEEK_MODEL` | 模型名 | `deepseek-v4-pro` |
| `DEEPSEEK_BASE_URL` | API 地址 | `https://api.deepseek.com/anthropic` |

## TUI 命令

| 命令 | 功能 |
|------|------|
| `/help` | 显示帮助 |
| `/clear` | 清屏 |
| `/echo <text>` | 回显测试 |
| `/save` | 保存当前会话 |
| `/history` | 列出历史会话 |
| `/load N` | 恢复历史会话 |
| `/quit` | 退出 |

## 代码风格

- Rust：`cargo fmt` + `cargo clippy`，错误处理用 `anyhow`
- Lua：snake_case，`host.*` 调用 Rust 工具函数
- Git：feat/fix/docs/refactor 前缀，提交信息用英文

## 重要说明

- Lua `system_core/` 支持自修改（edit_session 工具），`skills/` 不可修改
- 所有文件路径在 `safe_path()` 中做遍历检查
- TUI 自适应布局 + PageUp/Down 翻页 + 状态栏显示 token/缓存/上下文用量
- 流式输出：每 token 实时显示，支持明文 SSE 事件解析