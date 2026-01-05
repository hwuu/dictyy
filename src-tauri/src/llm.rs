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
        let config = {
            let lock = self.config.lock().unwrap();
            lock.clone().ok_or("LLM not configured")?
        };

        let prompt = format!(
            r#"请详细解释英语单词 "{}"，包括：
1. 音标（美式和英式）
2. 词性和中文释义
3. 常用例句（2-3个，带中文翻译）
4. 常用短语搭配
5. 词根词源（如有）
6. 记忆技巧

请用简洁清晰的格式回答。"#,
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

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(config.timeout))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

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

/// 获取配置文件路径
fn get_config_path() -> PathBuf {
    // 开发模式：从 src-tauri 目录读取
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.yaml");
    if dev_path.exists() {
        return dev_path;
    }

    // 生产模式：从可执行文件同级目录读取
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let prod_path = dir.join("config.yaml");
            if prod_path.exists() {
                return prod_path;
            }
        }
    }

    dev_path
}

/// 初始化 LLM
pub fn init_llm(app: &tauri::AppHandle) -> Result<(), String> {
    let config_path = get_config_path();
    let state = app.state::<LlmState>();
    state.init(config_path)
}

/// Tauri command: LLM 查询
#[tauri::command]
pub async fn llm_query(word: String, state: tauri::State<'_, LlmState>) -> Result<String, String> {
    state.query(&word).await
}
