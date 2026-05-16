# 对话持久化（SQLite）

> 当前状态：每次启动 Agent 都是全新上下文。
> 目标：自动保存对话历史，下次启动恢复，支持多会话管理。

## 方案设计

### 存储结构

使用 SQLite（零配置，单文件），存放在 `agent_plugins/memory/agent.db`。

```sql
-- 会话表
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,          -- UUID
    title TEXT,                   -- 第一条用户消息摘要
    created_at TEXT,              -- ISO 8601
    updated_at TEXT
);

-- 消息表
CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,           -- "user" | "assistant" | "tool"
    content TEXT NOT NULL,        -- JSON（Anthropic API 格式）
    step INTEGER,                 -- 消息序号
    created_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

### Rust 侧

**`src/persistence.rs`** — 新模块：

```rust
use rusqlite::Connection;
use serde_json::Value;

pub struct Session {
    pub id: String,
    pub title: String,
    pub messages: Vec<Value>,
}

pub fn init_db() -> anyhow::Result<Connection> {
    let db_path = crate::config::workspace_root()
        .join("agent_plugins/memory/agent.db");
    let conn = Connection::open(db_path)?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS sessions ...")?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS messages ...")?;
    Ok(conn)
}

pub fn save_session(conn: &Connection, session: &Session) -> anyhow::Result<()>
pub fn load_session(conn: &Connection, session_id: &str) -> anyhow::Result<Session>
pub fn list_sessions(conn: &Connection) -> anyhow::Result<Vec<(String, String)>>
```

### TUI 集成

启动时：

```
┌──────── Output ────────────┐
│  Previous sessions:        │
│  [1] 2026-05-16  pwd test │
│  [2] 2026-05-16  dep list │
│  [3] New session          │
│  Select session (1-3):    │
└────────────────────────────┘
```

按数字选择历史会话，或直接 Enter 开始新会话。

### Lua 侧

每次 `run_agent` 开始时从 `messages` 参数获取历史消息，结束后保存。

## 依赖

- `rusqlite = { version = "0.32", features = ["bundled"] }` 到 Cargo.toml

## 实现步骤

1. `Cargo.toml` 添加 `rusqlite`
2. 创建 `src/persistence.rs`
3. `main.rs` 启动时检查 db，提供会话列表
4. `run_agent_once` 接受 `Session` 参数
5. TUI 集成会话选择界面

预估工作量：**1 天**
