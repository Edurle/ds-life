# 文件监视器

> 当前：Agent 对项目变更无感知。
> 目标：自动检测 workspace 文件变化，Agent 通知用户。

## 方案

使用 `notify` crate 监听文件系统事件：

```rust
use notify::{Event, RecursiveMode, Watcher};

let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(move |res| {
    tx.send(res).unwrap();
})?;
watcher.watch(workspace_root, RecursiveMode::Recursive)?;

// 在 TUI 循环中非阻塞地检查
while let Ok(Ok(event)) = rx.try_recv() {
    // 追加到输出面板
    output_text.push_str(&format!("  [watch] {:?}\n", event.paths));
}
```

## TUI 表现

不打断 Agent 运行，在输出面板侧边显示文件变更通知：

```
┌──────── Output ────────────┐
│  (Agent thinking...)       │
│  [watch] src/main.rs       │  ← 文件变更通知
│  [watch] src/foo.rs        │
└────────────────────────────┘
```

## 高级：Agent 感知变更

将文件变更注入 Agent 消息上下文：

```lua
-- main_loop.lua 中检查是否有文件变更，若有则通过 user 消息通知 Agent
if host.file_events_pending() then
    table.insert(messages, {
        role = "user",
        content = "Files changed: src/main.rs, src/foo.rs"
    })
end
```

这样 Agent 在思考时可以自然地"看到"文件变更并反应。

## 依赖

- `notify = { version = "7", default-features = false, features = ["macos_kqueue"] }` （`"macos_kqueue"` 仅在 macOS 需要）

## 实现步骤

1. `Cargo.toml` 添加 `notify`
2. 创建 `src/file_watcher.rs`
3. 在 `main.rs` 启动时初始化 watcher
4. TUI 循环中检查事件
5. 可选：注入 Lua 侧

预估工作量：**0.5 天**
