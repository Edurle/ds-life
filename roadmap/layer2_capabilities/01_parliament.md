# 多 Agent 议会

## 概念

同时创建 3 个独立 Agent，各自不同角色，并行思考同一问题，最终"仲裁 Agent"综合结论。TUI 分栏展示各 Agent 的思维过程。

## 角色配置

```
[分析师]  system prompt: "你是技术分析师，深度分析问题，提供多角度视角"
[工程师]  system prompt: "你是一线工程师，给出可直接执行的方案和代码"
[测试员]  system prompt: "你是质量保证工程师，检查方案的边界条件和安全性"
[仲裁者]  system prompt: "综合三位专家的结论，给出最终建议"
```

## 并行执行

```rust
let (analyst, engineer, tester) = tokio::join!(
    spawn_agent("analyst", task),
    spawn_agent("engineer", task),
    spawn_agent("tester", task),
);
let verdict = await_arbiter(analyst, engineer, tester, task);
```

## TUI 分栏布局

```
┌──────────┬──────────┬──────────┐
│ 分析师   │ 工程师   │ 测试员   │
│ thinking │ checking │ reviewing│
│ ...      │ 方案 ok  │ 边界问题 │
├──────────┴──────────┴──────────┤
│        仲裁者综合              │
│    "分析师的 X 可行，但需处理  │
│     测试员提的 Y 边界"        │
└────────────────────────────────┘
```

## 挑战

- 多个 Agent 并行需要独立 Lua 环境（mlua 可创建多个实例）
- 3~4 倍 API 调用开销
- TUI 多栏布局需重构
