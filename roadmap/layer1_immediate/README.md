# Layer 1：立即可做的增强

改动量小（每个 0.5~1 天），但对用户体验提升显著。

## 子功能列表

| # | 功能 | 文件 | 状态 |
|---|------|------|------|
| 01 | 流式输出 | [01_streaming.md](01_streaming.md) | **已完成** |
| 02 | 对话持久化 | [02_persistence.md](02_persistence.md) | **已完成** |
| 03 | 命令系统（Slash 命令） | [03_shortcuts.md](03_shortcuts.md) | **已完成** |
| 04 | 文件监视器 | [04_file_watcher.md](04_file_watcher.md) | **标记为未来** |
| 05 | TUI 体验打磨 | [05_tui_ux.md](05_tui_ux.md) | **已完成** |

## 设计决策

| 问题 | 结论 |
|------|------|
| 快捷键 vs 命令 | **命令模式**（`/clear` 等），避免与 tmux/screen 冲突 |
| 内容溢出 | **PageUp/PageDown/Home/End 滚动** |
| 布局 | **自适应**（`Percentage(85)`） |
| 执行方式 | 命令行**不消耗 API Token** |
| 文件监视 | **跳过**，具体场景自动化更有价值，标记为未来方向 |
