# 第7步：崩溃检测与回滚

## 产出文件

| 文件 | 说明 |
|------|------|
| `agent_host/src/recovery.rs` | 滑动窗口崩溃检测器 |

## CrashDetector 设计

```rust
struct CrashDetector {
    window: VecDeque<bool>,         // 滑动窗口（默认 5 个记录）
    consecutive_failures: usize,   // 连续失败计数
}
```

### 触发条件（满足任一）

1. 滑动窗口中失败率 ≥ 60%（阈值可配置）
2. 连续失败 ≥ 3 次

### 修复要点

原计划中 `failure_count` 只增不减，导致达到阈值后永不下降。修复后：

- `record(true)` 时重置 `consecutive_failures = 0`
- 失败率从滑动窗口动态计算，成功记录会自然"推出"窗口

### 回滚流程

```
CrashDetector.record(success)
if should_rollback(0.6, 3):
    git reset --hard HEAD~1
    // 重建 Lua 环境
    restart agent
```

## 验证

```sh
cd agent_host && cargo build
```
