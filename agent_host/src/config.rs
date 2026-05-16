use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// 原始配置文件结构（对应 ~/.deepseek/config.toml）
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct DeepSeekConfig {
    api_key: Option<String>,
    default_text_model: Option<String>,
    reasoning_effort: Option<String>,
    base_url: Option<String>,
    projects: Option<HashMap<String, ProjectConfig>>,
}

/// 项目级配置
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ProjectConfig {
    trust_level: Option<String>,
}

/// 解析后供运行时使用的配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// API 密钥（最终有效值）
    pub api_key: String,
    /// 模型名
    pub model: String,
    /// API 基础 URL，会自动追加 `/v1/messages`
    pub base_url: String,
}

impl AppConfig {
    /// 加载配置：配置文件 + 环境变量覆盖
    ///
    /// 优先级：环境变量 > 配置文件 > 默认值
    pub fn load() -> anyhow::Result<Self> {
        let config = Self::load_toml();

        // API Key：环境变量优先
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
            .or_else(|_| {
                config
                    .as_ref()
                    .and_then(|c| c.api_key.clone())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "API key not found. Set ANTHROPIC_API_KEY / DEEPSEEK_API_KEY \
                             env var, or api_key in ~/.deepseek/config.toml"
                        )
                    })
            })?;

        // Model：DEEPSEEK_MODEL > ANTHROPIC_MODEL > 配置文件 default_text_model > 默认值
        let model = std::env::var("DEEPSEEK_MODEL")
            .or_else(|_| std::env::var("ANTHROPIC_MODEL"))
            .or_else(|_| -> Result<String, std::env::VarError> {
                config
                    .as_ref()
                    .and_then(|c| c.default_text_model.clone())
                    .ok_or_else(|| std::env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| "deepseek-v4-pro".to_string());

        // Base URL：DEEPSEEK_BASE_URL > ANTHROPIC_BASE_URL > 配置文件 base_url > 默认值
        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .or_else(|_| std::env::var("ANTHROPIC_BASE_URL"))
            .or_else(|_| -> Result<String, std::env::VarError> {
                config
                    .as_ref()
                    .and_then(|c| c.base_url.clone())
                    .ok_or_else(|| std::env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| "https://api.deepseek.com/anthropic".to_string());

        Ok(Self {
            api_key,
            model,
            base_url,
        })
    }

    /// 读取并解析 ~/.deepseek/config.toml
    fn load_toml() -> Option<DeepSeekConfig> {
        let config_path = config_path()?;
        let content = std::fs::read_to_string(&config_path).ok()?;
        match toml::from_str::<DeepSeekConfig>(&content) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!(
                    "Warning: failed to parse {}: {}",
                    config_path.display(),
                    e
                );
                None
            }
        }
    }
}

/// 获取配置文件路径：~/.deepseek/config.toml
fn config_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".deepseek").join("config.toml"))
}

/// 检测并返回工作区根目录。
///
/// 优先级：
/// 1. 环境变量 `AGENT_WORKSPACE`
/// 2. `CARGO_MANIFEST_DIR` 的父目录（若包含 agent_plugins）
/// 3. 当前目录（或父目录，若包含 agent_plugins）
/// 4. fallback 到当前目录
pub fn workspace_root() -> PathBuf {
    // 1. 环境变量
    if let Ok(root) = std::env::var("AGENT_WORKSPACE") {
        let p = PathBuf::from(&root);
        if p.exists() {
            return p;
        }
    }

    // 2. 编译时检测：CARGO_MANIFEST_DIR 的父目录
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        if let Some(parent) = PathBuf::from(&manifest).parent() {
            let parent = parent.to_path_buf();
            if parent.join("agent_plugins").is_dir() {
                return parent;
            }
        }
    }

    // 3. 运行时检测：当前目录或父目录
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("agent_plugins").is_dir() {
            return cwd;
        }
        if let Some(parent) = cwd.parent() {
            if parent.join("agent_plugins").is_dir() {
                return parent.to_path_buf();
            }
        }
    }

    // 4. fallback
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
