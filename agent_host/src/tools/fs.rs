use std::path::PathBuf;
use std::fs;

fn safe_path(path: &str) -> anyhow::Result<PathBuf> {
    let base = crate::config::workspace_root();

    // 先检查原始路径不包含明显的路径遍历
    if path.contains("..") {
        anyhow::bail!("Access denied: path contains '..'");
    }

    let full = base.join(path);

    // canonicalize 会解析符号链接，需二次验证
    let canonical = full.canonicalize()?;
    let base_canonical = base.canonicalize()?;
    if !canonical.starts_with(&base_canonical) {
        anyhow::bail!("Access denied: path escapes workspace");
    }

    Ok(canonical)
}

pub fn read_file(path: &str, limit: Option<usize>) -> anyhow::Result<String> {
    let path = safe_path(path)?;
    let content = fs::read_to_string(&path)?;
    let lines: Vec<&str> = content.lines().collect();
    if let Some(lim) = limit {
        if lim < lines.len() {
            let mut output = lines[..lim].join("\n");
            output.push_str(&format!("\n... ({} more lines)", lines.len() - lim));
            return Ok(output);
        }
    }
    Ok(content)
}

pub fn write_file(path: &str, content: &str) -> anyhow::Result<String> {
    let path = safe_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;
    Ok(format!("Wrote {} bytes to {}", content.len(), path.display()))
}

pub fn list_skills() -> anyhow::Result<Vec<String>> {
    let skills_dir = crate::config::workspace_root().join("agent_plugins/skills");
    let mut skills = Vec::new();
    if skills_dir.is_dir() {
        for entry in fs::read_dir(skills_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let skill_md = entry.path().join("SKILL.md");
                if skill_md.exists() {
                    skills.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(skills)
}
