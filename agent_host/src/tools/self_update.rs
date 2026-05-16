use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use std::process::Command;

lazy_static::lazy_static! {
    static ref SESSIONS: Mutex<HashMap<String, EditSession>> = Mutex::new(HashMap::new());
}

pub struct EditSession {
    pub id: String,
    /// 存储完整的相对路径（含 `agent_plugins/` 前缀），方便 git checkout
    pub files_changed: Vec<String>,
    pub base_commit: String,
}

fn git_commit(message: &str) -> anyhow::Result<()> {
    Command::new("git")
        .args(["add", "."])
        .status()?;
    Command::new("git")
        .args(["commit", "-m", message])
        .status()?;
    Ok(())
}

pub fn start_session() -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let base = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?
            .stdout,
    )?
    .trim()
    .to_string();

    let session = EditSession {
        id: id.clone(),
        files_changed: vec![],
        base_commit: base,
    };
    SESSIONS.lock().unwrap().insert(id.clone(), session);
    Ok(id)
}

pub fn update_plugin(
    session_id: &str,
    file_name: &str,
    content: &str,
) -> anyhow::Result<String> {
    let mut sessions = SESSIONS.lock().unwrap();
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

    // 限制：只允许修改 system_core/ 下的文件
    if !file_name.starts_with("system_core/") {
        anyhow::bail!(
            "Can only modify files under system_core/, got: {}",
            file_name
        );
    }

    let path = std::path::Path::new("agent_plugins").join(file_name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;

    // 用完整的相对路径（含 agent_plugins/），方便 git checkout
    let full_path = format!("agent_plugins/{}", file_name);
    session.files_changed.push(full_path);
    Ok(format!("Updated {}", file_name))
}

pub fn commit_session(session_id: &str, message: &str) -> anyhow::Result<String> {
    let sessions = SESSIONS.lock().unwrap();
    let session = sessions
        .get(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

    let full_message = format!("{} [session:{}]", message, session_id);
    git_commit(&full_message)?;
    Ok(format!("Committed session {}", session_id))
}

pub fn rollback_session(session_id: &str) -> anyhow::Result<String> {
    let mut sessions = SESSIONS.lock().unwrap();
    let session = sessions
        .remove(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

    for file in &session.files_changed {
        let status = Command::new("git")
            .args(["checkout", "HEAD", "--", file])
            .status()?;
        if !status.success() {
            eprintln!("Warning: failed to checkout {}", file);
        }
    }
    Ok("Rolled back".to_string())
}
