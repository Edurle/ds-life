# 快捷键系统

> 当前：TUI 只有 Esc 退出，Enter 发送。
> 目标：类 vim 快捷键，提升可用性。

## 快捷键映射

| 快捷键 | 功能 | 说明 |
|--------|------|------|
| `Ctrl+S` | 保存当前会话 | 若 persistence 已实现 |
| `Ctrl+L` | 清屏 | 清除输出面板 |
| `Ctrl+R` | 重放上一条消息 | 重新输入上次发送的消息 |
| `Ctrl+W` | 删除一个词 | 输入框中向后删除单词 |
| `Ctrl+A` | 跳到行首 | Home |
| `Ctrl+E` | 跳到行尾 | End |
| `Ctrl+P` / `↑` | 历史消息上翻 | 从历史列表中选择上一条 |
| `Ctrl+N` / `↓` | 历史消息下翻 | 从历史列表中选择下一条 |
| `Esc` | 退出 | 已有 |

## 实现

`src/tui.rs` 中 `Event::Key` 处理分支，扩展 `KeyCode` 匹配：

```rust
match key.code {
    KeyCode::Esc => break,
    KeyCode::Enter => { /* 发送 */ },
    KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
        // Ctrl+S: 保存
    },
    KeyCode::Char('l') if key.modifiers == KeyModifiers::CONTROL => {
        output_text.clear();  // Ctrl+L: 清屏
    },
    // ...
}
```

依赖 `KeyModifiers` 字段（`event::KeyEvent`）。

## 历史记录

内存中维护 `Vec<String>` history，Ctrl+P/N 翻找。

## 实现步骤

1. 扩展 `tui.rs` 的 key handling
2. 添加历史记录 Vec
3. 集成保存功能（依赖 02_persistence）
4. 编译 + TUI 交互测试

预估工作量：**0.5 天**
