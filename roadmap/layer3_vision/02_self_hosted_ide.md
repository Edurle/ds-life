# 自托管 IDE 模式

## 概念

TUI 升级为内置代码编辑面板（语法高亮、LSP 集成）。Agent 不只是跑命令，而是**在你面前实时编辑项目代码**。你可以审阅 diff、批准/拒绝修改。

## 布局

```
┌──────────┬─────────────────────┐
│ 终端输出 │ 代码编辑区          │
│ (Agent)  │ 📂 src/main.rs      │
│          │   fn main() {       │
│ Running  │     let x = 5;      │
│ tests... │ →     let y = 10;   │ ← Agent 修改
│          │   }                 │
├──────────┴─────────────────────┤
│ C-x approve  C-c reject        │
└────────────────────────────────┘
```

## 交互模式

- Agent 生成修改 → 编辑区显示 diff（颜色标记）
- 用户 Ctrl+X → 批准
- 用户 Ctrl+C → 拒绝
- 批准后 Agent 自动编译验证

## 技术挑战

- 语法高亮：需要 `syntect` 或 tree-sitter Rust binding
- LSP：集成 `lsp-server` 或通过子进程调用
- 代码编辑器：需要实现光标、选区、undo/redo
