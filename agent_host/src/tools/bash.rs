use std::process::Command;

/// `run_bash` 在 shell 中执行命令并返回输出。
///
/// # 安全注意
/// 黑名单匹配仅能阻止最明显的危险命令，可通过空格、引号、编码等方式绕过。
/// 生产环境应使用 seccomp 或容器隔离。
const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "sudo ",
    "shutdown",
    "reboot",
    "mkfs",
    "dd if=",
    "> /dev/",
    ":(){ :|:& };:",
    "chmod 777",
];

pub fn run_bash(command: &str) -> anyhow::Result<String> {
    // 不区分大小写检查
    let lower = command.to_lowercase();
    for pattern in BLOCKED_PATTERNS {
        if lower.contains(pattern) {
            anyhow::bail!(
                "Dangerous command blocked: matched '{}'",
                pattern
            );
        }
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut combined = format!("{}{}", stdout, stderr);

    if combined.len() > 50000 {
        combined = format!("{}... (truncated)", &combined[..50000]);
    }

    Ok(if combined.trim().is_empty() {
        "(no output)".into()
    } else {
        combined
    })
}
