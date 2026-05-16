# TUI 体验打磨：自适应布局 + 滚动翻页

> 当前问题：
> 1. 输出面板不随窗口大小变化，固定 `Min(3)` 高度
> 2. 内容超出屏幕高度后直接被截断，看不到历史
> 3. 无滚动机制

## 改进一：自适应布局

### 当前

```rust
.constraints([Constraint::Min(3), Constraint::Length(3)])
```

输入框固定 3 行，用户拉大窗口时输入框高度不变。

### 改为

```rust
.constraints([Constraint::Percentage(85), Constraint::Length(3)])
```

输出面板占 85% 高度，输入框固定 3 行。窗口变化时自动适配。

## 改进二：输出面板滚动

### 原理

`Paragraph` 的 `.scroll()` 接受 `(行偏移, 列偏移)`。内容超过面板高度时，设置偏移值滚动查看。

```rust
struct TuiState {
    output_text: String,
    scroll_offset: usize,
    input_buffer: String,
    processing: bool,
}
```

### 绘制

```rust
let max_lines = chunks[0].height as usize;
let total_lines = state.output_text.lines().count();
let scroll = state.scroll_offset.min(
    total_lines.saturating_sub(max_lines)
);
let output = Paragraph::new(state.output_text.clone())
    .scroll((scroll as u16, 0))
    .block(Block::default().title("Output").borders(Borders::ALL));
f.render_widget(output, chunks[0]);
```

### 翻页按键

| 按键 | 行为 | 说明 |
|------|------|------|
| `PageUp` | 向上翻一页 | 滚动面板高度行数 |
| `PageDown` | 向下翻一页 | 同上 |
| `Home` | 跳到顶部 | 回到最早输出 |
| `End` | 跳到底部 | 跟随最新输出（默认） |

```rust
KeyCode::PageUp => {
    let h = chunks[0].height as usize;
    state.scroll_offset = state.scroll_offset.saturating_add(h);
}
KeyCode::PageDown => {
    let h = chunks[0].height as usize;
    state.scroll_offset = state.scroll_offset.saturating_sub(h);
}
KeyCode::Home => state.scroll_offset = usize::MAX,
KeyCode::End => state.scroll_offset = 0,
```

### 自动跟随

默认 `scroll_offset = 0` 显示顶部内容。更好的默认行为是显示最新：

```rust
// 默认值：跳到末尾（最新的 Agent 输出）
state.scroll_offset = 0;
// 绘制时从底部计算偏移
// 或使用 Paragraph 的 scroll 属性直接控制
```

ratatui 的 `scroll((0, 0))` 表示不滚动（左上角）。我们需要使用 `scroll_offset` 计算反向偏移来显示最新内容。

## 改进三：输入框视觉

- processing 时标题栏改变（已有）
- 可增加输入框背景色变化提示

## 实现步骤

1. 变量集中到 `TuiState` 结构体
2. 布局改为 `Percentage(85) / Length(3)`
3. `Paragraph.scroll` 集成
4. PageUp/Down/Home/End 按键处理
5. 编译 + TUI 交互测试

预估工作量：**1 天**
