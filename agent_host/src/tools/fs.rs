use std::path::PathBuf;
use std::fs;
use std::io::Write;

fn safe_path(path: &str) -> anyhow::Result<PathBuf> {
    let base = crate::config::workspace_root();
    if path.contains("..") {
        anyhow::bail!("Access denied: path contains '..'");
    }
    let full = base.join(path);
    let canonical = full.canonicalize()?;
    let base_canonical = base.canonicalize()?;
    if !canonical.starts_with(&base_canonical) {
        anyhow::bail!("Access denied: path escapes workspace");
    }
    Ok(canonical)
}

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_LINE_LEN: usize = 2000;

/// 读取文件，带行号 + 大小/长度保护。
pub fn read_file(path: &str, limit: Option<usize>) -> anyhow::Result<String> {
    let path = safe_path(path)?;

    let meta = fs::metadata(&path)?;
    if meta.len() > MAX_FILE_SIZE {
        anyhow::bail!(
            "File too large: {:.1}MB (max {:.1}MB). Use a more specific path.",
            meta.len() as f64 / 1_000_000.0,
            MAX_FILE_SIZE as f64 / 1_000_000.0,
        );
    }

    let content = fs::read_to_string(&path).map_err(|e| {
        anyhow::anyhow!("Cannot read '{}': {}. Is it a text file?", path.display(), e)
    })?;

    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let digit_width = if total >= 10000 { 5 } else if total >= 1000 { 4 } else if total >= 100 { 3 } else { 2 };

    let take = limit.map(|l| l.min(total)).unwrap_or(total).min(total);
    let display = &lines[..take];

    let mut out = String::with_capacity(display.len() * 80);
    for (i, line) in display.iter().enumerate() {
        let n = i + 1;
        let text = if line.len() > MAX_LINE_LEN {
            format!("{} ...(truncated)", &line[..MAX_LINE_LEN])
        } else {
            (*line).to_string()
        };
        out.push_str(&format!("{:>width$}: {}\n", n, text, width = digit_width));
    }
    if take < total {
        out.push_str(&format!("... ({} more lines)\n", total - take));
    }

    Ok(if out.trim().is_empty() {
        "(empty file)".to_string()
    } else {
        out
    })
}

/// 写入文件，自动 .bak 备份 + 原子写入（tmp + rename）。
pub fn write_file(path: &str, content: &str) -> anyhow::Result<String> {
    let path = safe_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut info = String::new();
    if path.exists() {
        let changed = match fs::read_to_string(&path) {
            Ok(old) => old != content,
            Err(_) => true,
        };
        if changed {
            let bak = path.with_extension("bak");
            fs::copy(&path, &bak)?;
            info = format!(", backup at {}", bak.display());
        } else {
            return Ok(format!("File already up-to-date ({} bytes)", content.len()));
        }
    }

    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &path)?;

    Ok(format!("Wrote {} bytes to {}{}", content.len(), path.display(), info))
}

/// 对文件做行级精确修改。
///
/// `operations_json` 格式（JSON 数组）：
/// ```json
/// [
///   {"action":"replace","start_line":10,"end_line":12,"new_text":"fn main() {"},
///   {"action":"insert_after","line":5,"new_text":"let x = 1;"},
///   {"action":"insert_before","line":20,"new_text":"// TODO"},
///   {"action":"delete","start_line":30,"end_line":35}
/// ]
/// ```
/// 行号从 1 开始。操作按顺序执行。
pub fn apply_patch(path: &str, operations_json: &str) -> anyhow::Result<String> {
    let path = safe_path(path)?;
    let original = fs::read_to_string(&path)?;
    let mut lines: Vec<String> = original.lines().map(|l| l.to_string()).collect();
    let mut summary = Vec::new();

    let ops: Vec<serde_json::Value> = serde_json::from_str(operations_json)
        .map_err(|e| anyhow::anyhow!("Invalid operations JSON: {}", e))?;

    for (i, op) in ops.iter().enumerate() {
        let action = op["action"].as_str().unwrap_or("");
        let start = op["start_line"].as_u64().unwrap_or(0) as usize;
        let end = op["end_line"].as_u64().unwrap_or(0) as usize;
        let line = op["line"].as_u64().unwrap_or(0) as usize;
        let new_text = op["new_text"].as_str().unwrap_or("");

        match action {
            "replace" => {
                if start < 1 || end > lines.len() || start > end {
                    anyhow::bail!("Op {}: invalid range {}-{} (file {} lines)", i+1, start, end, lines.len());
                }
                let new: Vec<String> = new_text.lines().map(|l| l.to_string()).collect();
                let old_len = end - start + 1;
                lines.splice(start - 1..end, new.iter().cloned());
                summary.push(format!("replace {}-{} ({}→{} lines)", start, end, old_len, new.len()));
            }
            "delete" => {
                if start < 1 || end > lines.len() || start > end {
                    anyhow::bail!("Op {}: invalid delete range {}-{}", i+1, start, end);
                }
                let n = end - start + 1;
                lines.drain(start - 1..end);
                summary.push(format!("delete {}-{} ({} lines)", start, end, n));
            }
            "insert_after" => {
                if line > lines.len() { anyhow::bail!("Op {}: line {} out of range", i+1, line); }
                let new: Vec<String> = new_text.lines().map(|l| l.to_string()).collect();
                lines.splice(line..line, new.iter().cloned());
                summary.push(format!("insert {} lines after line {}", new.len(), line));
            }
            "insert_before" => {
                if line > lines.len() { anyhow::bail!("Op {}: line {} out of range", i+1, line); }
                let new: Vec<String> = new_text.lines().map(|l| l.to_string()).collect();
                lines.splice(line - 1..line - 1, new.iter().cloned());
                summary.push(format!("insert {} lines before line {}", new.len(), line));
            }
            _ => anyhow::bail!("Op {}: unknown action '{}'", i+1, action),
        }
    }

    let content = lines.join("\n") + "\n";
    let bak = path.with_extension("bak");
    fs::copy(&path, &bak)?;
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &path)?;

    Ok(format!("Patched {} ({} ops): {}", path.display(), ops.len(), summary.join("; ")))
}

pub fn list_skills() -> anyhow::Result<Vec<String>> {
    let skills_dir = crate::config::workspace_root().join("agent_plugins/skills");
    let mut skills = Vec::new();
    if skills_dir.is_dir() {
        for entry in fs::read_dir(skills_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                if entry.path().join("SKILL.md").exists() {
                    skills.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(skills)
}
