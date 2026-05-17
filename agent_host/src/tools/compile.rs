use serde_json::json;
use std::process::Command;

const RE_HEADER: &str = r"^(error|warning|note|help)(\[(?P<code>E\d+)\])?:\s*(?P<message>.+)$";
const RE_LOCATION: &str = r"^\s+-->\s+(?P<file>.+?):(?P<line>\d+):(?P<column>\d+)$";

/// 运行 `cargo build` 或 `cargo check`，返回结构化 JSON。
///
/// 输出包含 `errors`(结构数组) / `warnings` / `raw_stderr`(原始文本) / `success` / `exit_code`。
pub fn compile_with_feedback(crate_dir: &str, check_only: bool) -> anyhow::Result<String> {
    let cmd = if check_only { "check" } else { "build" };
    let output = Command::new("cargo")
        .args([cmd, "--color", "never"])
        .current_dir(crate_dir)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run cargo {}: {}", cmd, e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let success = output.status.success();
    let exit_code = output.status.code().unwrap_or(-1);

    let diag = parse_diagnostics(&stderr);
    let result = json!({
        "success": success,
        "exit_code": exit_code,
        "errors": diag.errors,
        "warnings": diag.warnings,
        "stdout": stdout.trim(),
        "raw_stderr": stderr.trim(),
    });

    Ok(serde_json::to_string_pretty(&result)?)
}

#[derive(Default)]
struct DiagResult {
    errors: Vec<serde_json::Value>,
    warnings: Vec<serde_json::Value>,
}

fn parse_diagnostics(stderr: &str) -> DiagResult {
    use regex::Regex;
    let re_h = Regex::new(RE_HEADER).unwrap();
    let re_l = Regex::new(RE_LOCATION).unwrap();

    let mut result = DiagResult::default();
    let mut entry: Option<serde_json::Map<String, serde_json::Value>> = None;
    let mut snippet: Vec<String> = Vec::new();
    let mut has_loc = false;

    for line in stderr.lines() {
        if let Some(caps) = re_h.captures(line) {
            if let Some(e) = entry.take() { flush_entry(&mut result, e, &snippet); snippet.clear(); has_loc = false; }

            let mut m = serde_json::Map::new();
            m.insert("level".into(), json!(caps.get(1).unwrap().as_str()));
            if let Some(c) = caps.name("code") { m.insert("code".into(), json!(c.as_str())); }
            m.insert("message".into(), json!(caps.name("message").unwrap().as_str()));
            m.insert("file".into(), json!(""));
            m.insert("line".into(), json!(0));
            m.insert("column".into(), json!(0));
            entry = Some(m);
            continue;
        }
        if let Some(caps) = re_l.captures(line) {
            if let Some(ref mut e) = entry {
                has_loc = true;
                e.insert("file".into(), json!(caps.name("file").unwrap().as_str()));
                e.insert("line".into(), json!(caps.name("line").unwrap().as_str().parse::<usize>().unwrap_or(0)));
                e.insert("column".into(), json!(caps.name("column").unwrap().as_str().parse::<usize>().unwrap_or(0)));
            }
            continue;
        }
        // 代码片段：数字行、| 行、= 帮助行
        if line.trim().starts_with(|c: char| c.is_ascii_digit()) || line.contains('|') || line.starts_with(" = ") {
            snippet.push(line.trim_end().to_string());
            continue;
        }
        if line.trim().is_empty() && has_loc { snippet.push(String::new()); }
    }

    if let Some(e) = entry.take() { flush_entry(&mut result, e, &snippet); }
    result
}

fn flush_entry(result: &mut DiagResult, mut entry: serde_json::Map<String, serde_json::Value>, snippet: &[String]) {
    let joined: String = snippet.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join("\n");
    if !joined.is_empty() { entry.insert("snippet".into(), json!(joined.trim_end())); }
    let level = entry.get("level").and_then(|v| v.as_str()).unwrap_or("");
    match level {
        "error" => result.errors.push(serde_json::Value::Object(entry)),
        "warning" => result.warnings.push(serde_json::Value::Object(entry)),
        _ => {}
    }
}
