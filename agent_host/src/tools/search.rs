use serde_json::json;
use std::fs;
use std::path::Path;

pub fn grep_files(
    pattern: &str, search_path: &str, include: &str,
    context_lines: usize, max_results: usize,
) -> anyhow::Result<String> {
    use regex::Regex;
    let re = Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
    let base = crate::config::workspace_root()
        .join(if search_path.is_empty() { "." } else { search_path })
        .canonicalize()?;
    let globs: Vec<&str> = if include.is_empty() { vec![] }
        else { include.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect() };
    let ctx = context_lines.max(1);
    let max = max_results.max(1);

    let mut results = Vec::new();
    let mut total = 0usize;
    walk_dir(&base, &globs, &base, &re, ctx, max, &mut results, &mut total)?;

    Ok(serde_json::to_string_pretty(&json!({
        "results": results, "total": total, "truncated": total > max,
    }))?)
}

fn walk_dir(
    dir: &Path, globs: &[&str], base: &Path, re: &regex::Regex,
    ctx: usize, max: usize, results: &mut Vec<serde_json::Value>, total: &mut usize,
) -> anyhow::Result<()> {
    if *total >= max { return Ok(()); }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = path.file_name().unwrap_or_default().to_string_lossy();
        if fname.starts_with('.') || fname == "target" || fname == "node_modules" { continue; }
        if path.is_dir() {
            walk_dir(&path, globs, base, re, ctx, max, results, total)?;
        } else if path.is_file() && (globs.is_empty() || matches_any(&fname, globs)) {
            if let Ok(content) = fs::read_to_string(&path) {
                grep_file(&path, &content, base, re, ctx, max, results, total)?;
            }
        }
    }
    Ok(())
}

fn matches_any(name: &str, globs: &[&str]) -> bool {
    for g in globs {
        if g.starts_with("*.") { if name.ends_with(&g[1..]) { return true; } }
        else if name.contains(g) { return true; }
    }
    false
}

fn grep_file(
    path: &Path, content: &str, base: &Path, re: &regex::Regex,
    ctx: usize, max: usize, results: &mut Vec<serde_json::Value>, total: &mut usize,
) -> anyhow::Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let rel = path.strip_prefix(base).unwrap_or(path);
    for (i, line) in lines.iter().enumerate() {
        if *total >= max { break; }
        if let Some(m) = re.find(line) {
            *total += 1;
            let s = i.saturating_sub(ctx);
            let e = (i + ctx + 1).min(lines.len());
            results.push(json!({
                "file": rel.to_string_lossy(),
                "line": i + 1,
                "column": m.start() + 1,
                "match": line,
                "before": &lines[s..i],
                "after": &lines[i+1..e],
            }));
        }
    }
    Ok(())
}

pub fn file_search(
    query: &str, search_path: &str, extensions: &str, max_results: usize,
) -> anyhow::Result<String> {
    let base = crate::config::workspace_root()
        .join(if search_path.is_empty() { "." } else { search_path })
        .canonicalize()?;
    let exts: Vec<&str> = if extensions.is_empty() { vec![] }
        else { extensions.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect() };
    let max = max_results.max(1);

    let mut files = Vec::new();
    collect_fs(&base, &exts, &base, &mut files)?;

    let q = query.to_lowercase();
    let mut scored: Vec<(f64, String)> = Vec::new();
    for f in &files {
        let p = f["path"].as_str().unwrap_or("");
        let s = fuzzy(&q, &p.to_lowercase());
        if s > 0.0 { scored.push((s, p.to_string())); }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max);

    Ok(serde_json::to_string_pretty(&json!({
        "files": scored.iter().map(|(s, p)| json!({"path": p, "score": *s})).collect::<Vec<_>>(),
        "total": files.len(),
    }))?)
}

fn collect_fs(dir: &Path, exts: &[&str], base: &Path, out: &mut Vec<serde_json::Value>) -> anyhow::Result<()> {
    let entries = match fs::read_dir(dir) { Ok(e) => e, Err(_) => return Ok(()), };
    for entry in entries {
        let entry = match entry { Ok(e) => e, Err(_) => continue };
        let path = entry.path();
        let fname = path.file_name().unwrap_or_default().to_string_lossy();
        if fname.starts_with('.') || fname == "target" || fname == "node_modules" { continue; }
        if path.is_dir() {
            collect_fs(&path, exts, base, out)?;
        } else if path.is_file() {
            if !exts.is_empty() {
                let ext = path.extension().unwrap_or_default().to_string_lossy();
                if !exts.contains(&ext.as_ref()) { continue; }
            }
            let rel = path.strip_prefix(base).unwrap_or(&path);
            out.push(json!({"path": rel.to_string_lossy()}));
        }
    }
    Ok(())
}

fn fuzzy(query: &str, target: &str) -> f64 {
    if target.contains(query) { return 0.8; }
    let mut qi = query.chars();
    let mut matched = 0;
    for tc in target.chars() {
        if let Some(qc) = qi.next() {
            if qc == tc { matched += 1; } else { if matched > 0 { break; } }
        } else { break; }
    }
    if matched > 0 { matched as f64 / query.len() as f64 * 0.5 } else { 0.0 }
}
