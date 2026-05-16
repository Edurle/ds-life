# 命令系统

> 原方案：快捷键（Ctrl+S/Ctrl+L/...）。
> 问题：Ctrl 组合键与 tmux、screen、readline 冲突严重。
> 改为：Slash 命令模式（类似 Claude Code 的 `/compact`），在输入框键入 `/xxx` 执行。

## 命令列表

| 命令 | 别名 | 功能 | 对应思路 |
|------|------|------|---------|
| `/clear` | `/c` | 清空输出面板 | Ctrl+L |
| `/save` | `/s` | 保存当前会话到 SQLite | Ctrl+S |
| `/history` | `/h`, `/hist` | 列出历史会话，`/h 2` 加载 | Ctrl+R |
| `/load N` | `/l N` | 加载第 N 个历史会话 | — |
| `/new` | `/n` | 新建空会话 | — |
| `/replay` | `/rp` | 重新发送上一次的消息 | Ctrl+P |
| `/help` | `/?` | 显示所有命令 | — |
| `/quit` | `/q`, `/exit` | 退出程序 | Esc |
| `/compact` | `/cmp` | 折叠输出面板早期内容 | — |

## 设计

### 输入处理

在 `tui.rs` 的 `Enter` 处理分支中：

```rust
if task.starts_with('/') {
    process_command(&task, &mut state).await;
} else {
    run_agent_once(&task).await;
}
```

### 输出

命令执行结果以 `#` 前缀追加到输出面板：

```
#  clear — 面板已清空
#  save — 会话保存成功
#  history — 找到 3 个历史会话
  [1] 2026-05-16  pwd test
  [2] 2026-05-16  dep list
#  load 2 — 已加载会话 #2
```

### 与普通消息区分

命令执行**不消耗 API Token**，全部由 Rust 侧处理。

## 实现

```rust
async fn process_command(cmd: &str, state: &mut TuiState) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts[0] {
        "/clear" | "/c" => state.output_text.clear(),
        "/save" | "/s"  => { /* persistence::save_session() */ },
        "/history" | "/h" | "/hist" => { /* 列出会话 */ },
        "/load" | "/l"  => { /* 加载指定会话 */ },
        "/new" | "/n"   => { /* 清空消息上下文 */ },
        "/help" | "/?"  => { /* 打印帮助 */ },
        "/quit" | "/q" | "/exit" => state.done = true,
        _ => state.output_text.push_str(&format!("# Unknown: {}\n", cmd)),
    }
}
```

## 优势对比

| 维度 | 快捷键方案 | Slash 命令方案 |
|------|-----------|---------------|
| tmux/screen 冲突 | Ctrl+S/A 会冲突 | 无 |
| 学习成本 | 需记住键位 | 可 /help 查看 |
| 可扩展性 | 有限组合键 | 任意命名 |
| 非技术用户 | 键位暗示不足 | 语义自解释 |

## 实现步骤

1. `tui.rs` 新增 `process_command` 函数
2. Enter 分支分流 `/xxx` 命令
3. 实现 /clear, /help, /quit
4. 输出样式 `# ` 前缀
5. 后续接入 persistence 后再实现 /save /history /load

预估工作量：**0.5 天**
