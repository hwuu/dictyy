//! LLM 模块 - 提供 LLM 回退查询功能

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

/// LLM 配置
#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_temperature() -> f32 { 0.3 }
fn default_max_tokens() -> u32 { 2048 }
fn default_timeout() -> u64 { 30 }

#[derive(Debug, Deserialize)]
struct ConfigFile {
    llm: LlmConfig,
}

/// OpenAI 兼容的请求格式
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// LLM 状态管理
pub struct LlmState {
    config: Mutex<Option<LlmConfig>>,
    client: Client,
}

impl LlmState {
    pub fn new() -> Self {
        Self {
            config: Mutex::new(None),
            client: Client::new(),
        }
    }

    /// 初始化配置
    pub fn init(&self, config_path: PathBuf) -> Result<(), String> {
        if !config_path.exists() {
            return Err(format!("Config file not found: {:?}", config_path));
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;

        let config_file: ConfigFile = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        let mut lock = self.config.lock().unwrap();
        *lock = Some(config_file.llm);
        Ok(())
    }

    /// 查询 LLM
    pub async fn query(&self, word: &str) -> Result<String, String> {
        crate::debug_log(&format!("[LLM] Starting query for: {}", word));

        let config = {
            let lock = self.config.lock().unwrap();
            lock.clone().ok_or("LLM not configured")?
        };

        crate::debug_log(&format!("[LLM] Config loaded - api_base: {}, model: {}, timeout: {}s",
            config.api_base, config.model, config.timeout));

        let prompt = format!(
            r#"请解释英语单词或短语 "{}"，返回 JSON 格式（不要包含 markdown 代码块标记）：

{{
  "phonetic_us": "美式音标，如无则为 null",
  "phonetic_uk": "英式音标，如无则为 null",
  "translations": [
    {{ "pos": "词性（如 n. / v. / adj.）", "tranCn": "中文释义" }}
  ],
  "sentences": [
    {{ "en": "英文例句", "cn": "中文翻译" }}
  ],
  "phrases": [
    {{ "phrase": "短语", "meaning": "含义" }}
  ],
  "rememberMethod": "记忆技巧或词源说明，如无则为 null"
}}

要求：
1. translations 至少包含 1 个释义
2. sentences 包含 2-3 个例句
3. phrases 包含常用短语搭配（如有），否则为空数组
4. 只返回 JSON，不要其他内容"#,
            word
        );

        let request = ChatRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        };

        let url = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));
        crate::debug_log(&format!("[LLM] Sending request to: {}", url));

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(config.timeout))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                crate::debug_log(&format!("[LLM] Request error: {:?}", e));
                format!("Request failed: {}", e)
            })?;

        crate::debug_log(&format!("[LLM] Response status: {}", response.status()));

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, text));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or("No response from LLM".to_string())
    }
}

/// 获取默认配置模板路径
fn get_default_config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    // 从资源目录获取模板
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidates = [
            resource_dir.join("config.yaml.example"),
            resource_dir.join("resources").join("config.yaml.example"),
        ];
        for path in &candidates {
            if path.exists() {
                return Some(path.clone());
            }
        }
    }

    // 开发模式：检查当前工作目录
    let dev_candidates = [
        std::env::current_dir().ok().map(|p| p.join("src-tauri").join("config.yaml.example")),
        std::env::current_dir().ok().map(|p| p.join("config.yaml.example")),
    ];
    for path_opt in &dev_candidates {
        if let Some(path) = path_opt {
            if path.exists() {
                return Some(path.clone());
            }
        }
    }

    None
}

/// 确保用户配置目录存在，如果配置不存在则复制模板
fn ensure_config(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 获取用户本地数据路径 (%LOCALAPPDATA%\Dictyy)
    let config_dir = dirs::data_local_dir()
        .ok_or("Cannot determine local data directory")?
        .join("Dictyy");

    crate::debug_log(&format!("[LLM] Config dir: {:?}", config_dir));

    // 创建目录
    if !config_dir.exists() {
        crate::debug_log("[LLM] Creating config dir...");
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config_path = config_dir.join("config.yaml");
    crate::debug_log(&format!("[LLM] Config path: {:?}, exists: {}", config_path, config_path.exists()));

    // 如果配置不存在，复制模板
    if !config_path.exists() {
        if let Some(template_path) = get_default_config_path(app) {
            crate::debug_log(&format!("[LLM] Copying template from {:?}", template_path));
            fs::copy(&template_path, &config_path)
                .map_err(|e| format!("Failed to copy config template: {}", e))?;
        } else {
            // 没有模板，创建默认配置
            crate::debug_log("[LLM] No template found, creating default config");
            let default_config = r#"# Dictyy LLM 配置文件
# 请填写您的 API 配置

llm:
  api_base: "https://api.example.com/v1"
  api_key: "your-api-key-here"
  model: "model-name"
  temperature: 0.3
  max_tokens: 2048
  timeout: 30
"#;
            fs::write(&config_path, default_config)
                .map_err(|e| format!("Failed to create default config: {}", e))?;
        }
    }

    Ok(config_path)
}

/// 初始化 LLM
pub fn init_llm(app: &tauri::AppHandle) -> Result<(), String> {
    crate::debug_log("[LLM] Initializing...");

    // 开发模式：检查当前工作目录下的 config.yaml
    let dev_path = std::env::current_dir()
        .ok()
        .map(|p| p.join("src-tauri").join("config.yaml"));

    let config_path = if let Some(ref path) = dev_path {
        if path.exists() {
            crate::debug_log(&format!("[LLM] Using dev config: {:?}", path));
            path.clone()
        } else {
            // 生产模式：确保用户配置存在
            ensure_config(app)?
        }
    } else {
        ensure_config(app)?
    };

    crate::debug_log(&format!("[LLM] Using config: {:?}", config_path));

    let state = app.state::<LlmState>();
    state.init(config_path.clone()).map_err(|e| {
        crate::debug_log(&format!("[LLM] Init failed: {}", e));
        e
    })?;

    crate::debug_log("[LLM] Successfully initialized");
    Ok(())
}

/// Tauri command: LLM 查询
#[tauri::command]
pub async fn llm_query(word: String, state: tauri::State<'_, LlmState>) -> Result<String, String> {
    state.query(&word).await
}

/// LLM 配置信息（用于前端显示）
#[derive(Debug, Serialize)]
pub struct LlmConfigInfo {
    pub api_base: String,
    pub model: String,
    pub configured: bool,
}

/// Tauri command: 获取 LLM 配置信息
#[tauri::command]
pub fn get_llm_config(state: tauri::State<'_, LlmState>) -> LlmConfigInfo {
    let lock = state.config.lock().unwrap();
    match lock.as_ref() {
        Some(config) => LlmConfigInfo {
            api_base: config.api_base.clone(),
            model: config.model.clone(),
            configured: true,
        },
        None => LlmConfigInfo {
            api_base: String::new(),
            model: String::new(),
            configured: false,
        },
    }
}
