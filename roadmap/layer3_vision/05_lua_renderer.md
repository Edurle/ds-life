# Lua 驱动的可组合渲染引擎

## 概念

将 TUI 的渲染逻辑从 Rust 硬编码变为 Lua 脚本驱动。Rust 提供底层渲染原语（draw_text、draw_box 等），Lua 脚本负责每帧的组合和布局。

## 架构

```
┌─ Rust (渲染引擎) ────────────────┐
│                                  │
│  每帧：                           │
│    1. host.clear_canvas()        │
│    2. host.run("layout.lua")     │
│    3. 消费 draw_calls 队列        │
│    4. 批量渲染到 ratatui          │
│                                  │
│  渲染原语：                       │
│    host.draw_text(x, y, text)    │
│    host.draw_box(x, y, w, h)     │
│    host.draw_line(x1, y1, x2,...)│
│    host.set_style(color, bold)   │
│    host.get_term_size()          │
│    host.bind_key(key, callback)  │
└──────────────────────────────────┘
         ▲
         │ Lua → Rust
         ▼
┌─ Lua (渲染逻辑) ─────────────────┐
│                                  │
│  function render_frame()         │
│    local w, h = host.get_term()  │
│    host.draw_box(1, 1, w, h-4)   │
│    host.draw_text(2, 2, output)  │
│    host.draw_box(1, h-3, w, 2)   │
│    host.draw_text(2, h-2, input) │
│    host.draw_line(1, h, w, info) │
│  end                             │
│                                  │
│  host.on_frame(render_frame)     │
└──────────────────────────────────┘
```

## 优势

| 维度 | Rust 硬编码 | Lua 渲染 |
|------|-----------|----------|
| 修改布局 | 改 Rust 重新编译 | 改 .lua 热重载 |
| Agent 自定制 | 不能 | Agent 可修改 layout.lua |
| 技能扩展 | 不能 | 技能包可注册面板 |
| 复杂组件 | 每个要改 Rust | Lua 组合，无需编译 |

## 触发场景

- 技能需要定制 UI（文件树浏览器）
- Agent 工厂生成的 Agent 需要自己的布局
- 自托管 IDE 的三栏布局
- 多 Agent 议会的分栏展示

## 实现路径

1. Rust 侧实现 draw_calls 队列，Lua 原语 push 操作，帧末批量消费
2. 默认 layout.lua 保持与当前 TUI 一致的界面
3. 热重载：检测 layout.lua 变更后自动重载渲染函数
4. 技能包可挂载自己的渲染脚本到指定区域

## 挑战

- 跨边界调用频率高，需要批量处理
- 按键事件仍在 Rust 侧处理，通过 host.bind_key 注册 Lua 回调
- 滚动、焦点等交互逻辑的划分