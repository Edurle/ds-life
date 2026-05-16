# 第2步：构建沙箱和基础安全工具

## 产出文件

| 文件 | 说明 |
|------|------|
| `agent_host/src/tools/mod.rs` | 模块入口 |
| `agent_host/src/tools/fs.rs` | 安全文件读写 + list_skills |
| `agent_host/src/tools/bash.rs` | 安全 shell 执行 |
| `agent_host/src/tools/llm.rs` | LLM API 调用（含 tools 定义 + system prompt + 重试） |

## 关键设计

### safe_path（fs.rs）
- 拒绝 `..` 路径遍历
- 两次 `canonicalize` 验证，防止符号链接转义
- 仅限工作区内操作

### bash 安全（bash.rs）
- 黑名单匹配危险命令模式
- **注意**：黑名单模式不完善，生产环境应用 seccomp/容器隔离

### llm_chat（llm.rs）
- 固定 `SYSTEM_PROMPT` 指导 Agent 行为
- 固定 `TOOLS_DEFINITION` 告知 API 可用工具
- 指数退避重试（最多 3 次）

## 验证

```sh
cd agent_host && cargo build
# 应编译成功，无错误
```
