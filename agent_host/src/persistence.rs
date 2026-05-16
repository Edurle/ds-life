use rusqlite::{params, Connection};
use serde_json::Value;

/// 一个对话会话。
#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    /// 按顺序存储的 messages（Anthropic API 格式）
    pub messages: Vec<Value>,
}

/// 初始化数据库，创建表（如不存在）。
pub fn init_db() -> anyhow::Result<Connection> {
    let db_path = crate::config::workspace_root()
        .join("agent_plugins/memory/agent.db");

    // 确保父目录存在
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(&db_path)?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id         TEXT PRIMARY KEY,
            title      TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role       TEXT NOT NULL,
            content    TEXT NOT NULL,
            step       INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session
            ON messages(session_id, step);
        ",
    )?;

    Ok(conn)
}

/// 创建一个新会话，返回 session_id。
pub fn create_session(conn: &Connection, title: &str) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = iso_now();
    conn.execute(
        "INSERT INTO sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, title, now, now],
    )?;
    Ok(id)
}

/// 向会话追加一条消息。
pub fn append_message(
    conn: &Connection,
    session_id: &str,
    role: &str,
    content: &str,
    step: i32,
) -> anyhow::Result<()> {
    let now = iso_now();
    conn.execute(
        "INSERT INTO messages (session_id, role, content, step, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, role, content, step, now],
    )?;
    // 更新会话时间戳
    conn.execute(
        "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
        params![now, session_id],
    )?;
    Ok(())
}

/// 列出所有会话（id + 标题 + 时间 + 消息数）。
pub fn list_sessions(conn: &Connection) -> anyhow::Result<Vec<(String, String, String, i32)>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.title, s.updated_at,
                (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id) as msg_count
         FROM sessions s
         ORDER BY s.updated_at DESC
         LIMIT 20",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;

    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row?);
    }
    Ok(sessions)
}

/// 加载一个会话的全部消息。
pub fn load_messages(conn: &Connection, session_id: &str) -> anyhow::Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT role, content FROM messages
         WHERE session_id = ?1
         ORDER BY step ASC",
    )?;

    let rows = stmt.query_map(params![session_id], |row| {
        let role: String = row.get(0)?;
        let content: String = row.get(1)?;
        // 尝试解析 content 为 JSON，若失败则当作纯文本
        let content_val: Value = serde_json::from_str(&content).unwrap_or(Value::String(content));
        let msg = serde_json::json!({
            "role": role,
            "content": content_val,
        });
        Ok(msg)
    })?;

    let mut messages = Vec::new();
    for row in rows {
        messages.push(row?);
    }
    Ok(messages)
}

/// 获取当前 ISO 时间戳字符串。
fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // 简易格式化：YYYY-MM-DD HH:MM:SS
    let secs = d.as_secs();
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;

    // 日期从 Unix epoch 1970-01-01 算起
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut mo = 1usize;
    for &md in month_days.iter() {
        if remaining < md {
            break;
        }
        remaining -= md;
        mo += 1;
    }
    let day = remaining + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y, mo, day, h, m, s
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
