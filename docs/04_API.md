# 第4步：连接真实 Anthropic API

## 配置系统

### 方式一：配置文件（推荐）

编辑 `~/.deepseek/config.toml`：

```toml
api_key = "sk-你的密钥"
default_text_model = "deepseek-v4-pro"
base_url = "https://api.deepseek.com/anthropic"

# 可选：推理强度
reasoning_effort = "auto"
```

`base_url` 会被自动拼接 `/v1/messages` 作为 API 端点。

### 方式二：环境变量

| 环境变量 | 说明 | 示例 |
|---------|------|------|
| `ANTHROPIC_API_KEY` | API Key（兼容） | `sk-ant-xxx` |
| `DEEPSEEK_API_KEY` | API Key（替代） | `sk-xxx` |
| `ANTHROPIC_BASE_URL` | 基础 URL（兼容） | `https://api.deepseek.com/anthropic` |
| `DEEPSEEK_BASE_URL` | 基础 URL（替代） | `https://api.deepseek.com/anthropic` |
| `ANTHROPIC_MODEL` | 模型名（兼容） | `deepseek-v4-pro` |
| `DEEPSEEK_MODEL` | 模型名（替代） | `deepseek-v4-pro` |

### 优先级

```
环境变量 > 配置文件 > 默认值
```

- API key：`ANTHROPIC_API_KEY` > `DEEPSEEK_API_KEY` > `config.toml api_key` > 报错
- Model：`DEEPSEEK_MODEL` > `ANTHROPIC_MODEL` > `config.toml default_text_model` > `deepseek-v4-pro`
- Base URL：`DEEPSEEK_BASE_URL` > `ANTHROPIC_BASE_URL` > `config.toml base_url` > `https://api.deepseek.com/anthropic`

### 默认值

- **base_url**: `https://api.deepseek.com/anthropic`（DeepSeek 的 Anthropic API 兼容端点）
- **model**: `deepseek-v4-pro`

## llm_chat 请求体

Rust 侧组装完整请求体，包含：

```json
{
    "model": "deepseek-v4-pro",
    "max_tokens": 4096,
    "system": "You are a self-iterating coding agent...",
    "tools": [...],
    "messages": [...]
}
```

请求发送到 `{base_url}/v1/messages`。

## 重试逻辑

- 最多 3 次
- `429 Rate Limited` → 指数退避 (2s, 4s, 8s)
- 网络错误 → 指数退避重试

## 验证

```sh
export ANTHROPIC_API_KEY="你的密钥"
# 或者编辑 ~/.deepseek/config.toml
cd agent_host && cargo build
```
