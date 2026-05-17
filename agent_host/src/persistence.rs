use rusqlite::{params, Connection};
use serde_json::Value;

#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub workspace: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<Value>,
}

pub fn init_db() -> anyhow::Result<Connection> {
    let workspace = crate::config::workspace_root();
    let db_path = workspace.join("agent_plugins/memory/agent.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&db_path)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id         TEXT PRIMARY KEY,
            title      TEXT NOT NULL,
            workspace  TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );",
    )?;
    let has_ws: bool = conn.prepare("SELECT workspace FROM sessions LIMIT 0").is_ok();
    if !has_ws {
        let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN workspace TEXT NOT NULL DEFAULT ''");
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS messages (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role       TEXT NOT NULL,
            content    TEXT NOT NULL,
            step       INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );
        CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, step);",
    )?;
    Ok(conn)
}

pub fn current_workspace() -> String {
    crate::config::workspace_root().to_string_lossy().to_string()
}

pub fn create_session(conn: &Connection, title: &str) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = iso_now();
    let ws = current_workspace();
    conn.execute(
        "INSERT INTO sessions (id, title, workspace, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, title, ws, now, now],
    )?;
    Ok(id)
}

pub fn append_message(
    conn: &Connection, session_id: &str, role: &str, content: &str, step: i32,
) -> anyhow::Result<()> {
    let now = iso_now();
    conn.execute(
        "INSERT INTO messages (session_id, role, content, step, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, role, content, step, now],
    )?;
    conn.execute("UPDATE sessions SET updated_at = ?1 WHERE id = ?2", params![now, session_id])?;
    Ok(())
}

pub fn list_sessions(conn: &Connection) -> anyhow::Result<Vec<(String, String, String, i32)>> {
    let ws = current_workspace();
    let mut stmt = conn.prepare(
        "SELECT s.id, s.title, s.updated_at,
                (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id) as msg_count
         FROM sessions s WHERE s.workspace = ?1
         ORDER BY s.updated_at DESC LIMIT 20",
    )?;
    let rows = stmt.query_map(params![ws], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i32>(3)?))
    })?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn load_messages(conn: &Connection, session_id: &str) -> anyhow::Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT role, content FROM messages WHERE session_id = ?1 ORDER BY step ASC",
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        let role: String = row.get(0)?;
        let content: String = row.get(1)?;
        let cv: Value = serde_json::from_str(&content).unwrap_or(Value::String(content));
        Ok(serde_json::json!({"role": role, "content": cv}))
    })?;
    let mut msgs = Vec::new();
    for r in rows { msgs.push(r?); }
    Ok(msgs)
}

pub fn save_messages(conn: &Connection, session_id: &str, messages: &[Value]) -> anyhow::Result<()> {
    conn.execute("DELETE FROM messages WHERE session_id = ?1", params![session_id])?;
    for (i, msg) in messages.iter().enumerate() {
        let role = msg["role"].as_str().unwrap_or("unknown");
        let content = serde_json::to_string(&msg["content"]).unwrap_or_default();
        append_message(conn, session_id, role, &content, i as i32 + 1)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn delete_session(conn: &Connection, session_id: &str) -> anyhow::Result<()> {
    conn.execute("DELETE FROM messages WHERE session_id = ?1", params![session_id])?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
    Ok(())
}

fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let s = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let days = s / 86400; let t = s % 86400;
    let h = t / 3600; let m = (t % 3600) / 60; let sec = t % 60;
    let mut y = 1970i64; let mut rem = days as i64;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if rem < dy { break; } rem -= dy; y += 1;
    }
    let md: &[i64] = if is_leap(y) { &[31,29,31,30,31,30,31,31,30,31,30,31] } else { &[31,28,31,30,31,30,31,31,30,31,30,31] };
    let mut mo = 1;
    for &d in md { if rem < d { break; } rem -= d; mo += 1; }
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, rem+1, h, m, sec)
}

fn is_leap(y: i64) -> bool { (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 }
