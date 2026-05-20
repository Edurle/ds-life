use serde_json::json;
use std::process::Command;

/// 返回结构化 Git 状态 + diff。
///
/// 返回 JSON:
/// ```json
/// {
///   "branch": "master",
///   "dirty": true,
///   "changes": [
///     {"file": "src/main.rs", "status": "M", "staged": false, "insertions": 12, "deletions": 3}
///   ],
///   "summary": "1 file changed, 12 insertions(+), 3 deletions(-)",
///   "diff": "--- a/src/main.rs\n+++ b/src/main.rs\n..."
/// }
/// ```
pub fn git_changes() -> anyhow::Result<String> {
    let workspace = crate::config::workspace_root();

    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&workspace)
        .output()?;
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();

    // 检查是否有未提交改动
    let dirty = !Command::new("git")
        .args(["diff", "--quiet"])
        .current_dir(&workspace)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // 结构化解析 git status --porcelain
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&workspace)
        .output()?;
    let status_out = String::from_utf8_lossy(&out.stdout);

    let mut changes: Vec<serde_json::Value> = Vec::new();
    for line in status_out.lines() {
        if line.len() < 3 {
            continue;
        }
        let xy = &line[..2];
        let file = line[3..].trim();
        let staged = !xy.starts_with(' ');
        // 提取状态：M=modified, A=added, D=deleted, ??=untracked, R=renamed
        let status = if xy.starts_with('?') {
            "??"
        } else {
            let x = xy.chars().next().unwrap_or(' ');
            let y = xy.chars().nth(1).unwrap_or(' ');
            let s = if y != ' ' { y } else { x };
            match s {
                'M' => "M", 'A' => "A", 'D' => "D", 'R' => "R", _ => "?",
            }
        };
        changes.push(json!({"file": file, "status": status, "staged": staged}));
    }

    // --stat 摘要
    let out = Command::new("git")
        .args(["diff", "--stat", "--color=never"])
        .current_dir(&workspace)
        .output()?;
    let summary = String::from_utf8_lossy(&out.stdout).trim().to_string();

    let out = Command::new("git")
        .args(["diff", "--color=never"])
        .current_dir(&workspace)
        .output()?;
    let diff = String::from_utf8_lossy(&out.stdout).trim().to_string();

    Ok(serde_json::to_string_pretty(&json!({
        "branch": branch,
        "dirty": dirty,
        "changes": changes,
        "summary": summary,
        "diff": diff,
    }))?)
}
