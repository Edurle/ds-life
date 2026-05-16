# 第9步：TUI 构建

## 产出文件

| 文件 | 说明 |
|------|------|
| `agent_host/src/tui.rs` | 终端 UI 界面 |

## 依赖

已在 `Cargo.toml` 中声明 `ratatui` 和 `crossterm`。

## 布局

```
┌──────── Output ────────────┐
│                            │
│  Agent TUI ready.          │
│  > ls -la                  │
│  (Agent processing...)     │
│                            │
├──────── Input ─────────────┤
│  my next command           │
└────────────────────────────┘
```

## 交互

| 按键 | 行为 |
|------|------|
| 字符键 | 输入消息 |
| Enter | 发送到 Agent |
| Backspace | 删除字符 |
| Esc | 退出 |

## 修复要点

原计划使用 `Text::from()` 尝试渲染为 Widget 失败。修复后使用 `Paragraph::new()`（实现了 `Widget` trait）。

## 模式切换

```sh
cargo run        # CLI 模式
cargo run tui    # TUI 模式（通过 args[1] 判断）
```
