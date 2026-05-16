# 第6步：自我迭代机制

## 产出文件

| 文件 | 说明 |
|------|------|
| `agent_host/src/tools/self_update.rs` | 编辑会话管理 |
| `agent_plugins/system_core/self_improve.lua` | 自我迭代函数 |

## 编辑会话工作流

```
1. start_session() → session_id
2. update_plugin(session_id, "system_core/xxx.lua", new_code)
   ├── 校验: file_name 必须以 "system_core/" 开头
   └── 限制: 只允许修改 agent_plugins 内文件
3. commit_session(session_id, "message")  → git commit
   或
   rollback_session(session_id) → git checkout HEAD -- 文件
```

## 自我迭代流程

1. Agent 读取自身 `main_loop.lua` 源码
2. 调用 LLM 要求"改进代码"
3. 用 `load()` 对生成代码做语法验证
4. 通过后创建编辑会话 → 更新文件 → commit

## 安全措施

- 只允许修改 `system_core/` 下的文件
- 新代码必须通过 Lua `load()` 语法检查（自我迭代拒绝语法错误的代码）
- 每次修改独立 commit，可单独回滚
- 回滚使用 `git checkout HEAD --` 而非 `git reset`，保留提交历史

## 验证

```sh
cd agent_host && cargo build
# 应编译成功
```
