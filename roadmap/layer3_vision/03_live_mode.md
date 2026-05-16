# 实时协奏模式

## 概念

Agent 不再是"一问一答"。它持续在后台运行，监听你的终端操作，自动提供建议。

## 场景

```
你在终端: vim src/main.rs
Agent 侧栏: "这个函数缺少错误处理，建议加 match"
           : "上次你改这个文件时引入了 bug，需要我帮你查吗？"

你在终端: git commit -m "fix"
Agent 侧栏: "commit message 太简略，建议改为 fix: handle null ptr"
           : (自动) git commit --amend -m "fix: handle null pointer..."
```

## 实现

- 终端监听通过 `pty` 代理或 `inotify` 监听 `.bash_history`
- Agent 在后台持续评估当前上下文
- 侧栏模式：不打断用户，右方侧栏默默提示
- 用户可按 Tab 关注 Agent 建议，或继续忽略

## 挑战

- 隐私：监听终端操作需用户明确授权
- 时机：延迟太高会影响"实时"感
- 正确性：如何避免噪音建议
