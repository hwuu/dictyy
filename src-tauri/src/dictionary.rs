//! 词典模块 - 提供离线词典查询功能

use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use strsim::levenshtein;
use tauri::{AppHandle, Manager};

/// 词典查询结果
#[derive(Debug, Serialize, Deserialize)]
pub struct WordEntry {
    pub word: String,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    pub content: String,
    pub sources: Vec<String>,
    pub gpt4_content: Option<String>,
}

/// 搜索建议结果
#[derive(Debug, Serialize, Deserialize)]
pub struct WordSuggestion {
    pub word: String,
    pub brief: String, // 简短释义
}

/// 词典状态管理
pub struct DictionaryState {
    conn: Mutex<Option<Connection>>,
}

impl DictionaryState {
    pub fn new() -> Self {
        Self {
            conn: Mutex::new(None),
        }
    }

    /// 初始化数据库连接
    pub fn init(&self, db_path: PathBuf) -> SqliteResult<()> {
        let conn = Connection::open(db_path)?;
        let mut lock = self.conn.lock().unwrap();
        *lock = Some(conn);
        Ok(())
    }

    /// 查询单词
    pub fn lookup(&self, word: &str) -> SqliteResult<Option<WordEntry>> {
        let lock = self.conn.lock().unwrap();
        let conn = match lock.as_ref() {
            Some(c) => c,
            None => return Ok(None),
        };

        // 查询主词典
        let mut stmt = conn.prepare(
            "SELECT w.word, w.phonetic_us, w.phonetic_uk, w.content, g.content as gpt4_content
             FROM words w
             LEFT JOIN gpt4_words g ON LOWER(w.word) = LOWER(g.word)
             WHERE LOWER(w.word) = LOWER(?1)"
        )?;

        let result = stmt.query_row([word], |row| {
            Ok(WordEntry {
                word: row.get(0)?,
                phonetic_us: row.get(1)?,
                phonetic_uk: row.get(2)?,
                content: row.get(3)?,
                sources: vec![],
                gpt4_content: row.get(4)?,
            })
        });

        match result {
            Ok(mut entry) => {
                // 查询词典来源
                let mut sources_stmt = conn.prepare(
                    "SELECT ws.source FROM word_sources ws
                     JOIN words w ON ws.word_id = w.id
                     WHERE LOWER(w.word) = LOWER(?1)"
                )?;
                let sources: Vec<String> = sources_stmt
                    .query_map([word], |row| row.get(0))?
                    .filter_map(|r| r.ok())
                    .collect();
                entry.sources = sources;
                Ok(Some(entry))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 搜索单词（模糊匹配）
    pub fn search(&self, query: &str, limit: usize) -> SqliteResult<Vec<WordSuggestion>> {
        let lock = self.conn.lock().unwrap();
        let conn = match lock.as_ref() {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let query_lower = query.to_lowercase();

        // 查询前缀匹配的单词
        let mut stmt = conn.prepare(
            "SELECT word, content FROM words
             WHERE LOWER(word) LIKE ?1
             ORDER BY LENGTH(word) ASC
             LIMIT 50"
        )?;

        let pattern = format!("{}%", query_lower);
        // (word, content, is_prefix, distance)
        let mut candidates: Vec<(String, String, bool, usize)> = stmt
            .query_map([&pattern], |row| {
                let word: String = row.get(0)?;
                let content: String = row.get(1)?;
                Ok((word, content))
            })?
            .filter_map(|r| r.ok())
            .map(|(word, content)| {
                let distance = levenshtein(&query_lower, &word.to_lowercase());
                (word, content, true, distance) // true = 前缀匹配
            })
            .collect();

        // 如果前缀匹配结果不足，再查询编辑距离相近的词
        if candidates.len() < limit {
            let mut stmt2 = conn.prepare(
                "SELECT word, content FROM words
                 WHERE LOWER(word) NOT LIKE ?1
                 AND LENGTH(word) BETWEEN ?2 AND ?3
                 LIMIT 100"
            )?;

            let min_len = query.len().saturating_sub(1);
            let max_len = query.len() + 2;

            let additional: Vec<(String, String, bool, usize)> = stmt2
                .query_map(rusqlite::params![&pattern, min_len, max_len], |row| {
                    let word: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    Ok((word, content))
                })?
                .filter_map(|r| r.ok())
                .map(|(word, content)| {
                    let distance = levenshtein(&query_lower, &word.to_lowercase());
                    (word, content, false, distance) // false = 非前缀匹配
                })
                .filter(|(_, _, _, distance)| *distance <= 2) // 更严格：编辑距离 <= 2
                .collect();

            candidates.extend(additional);
        }

        // 排序：前缀匹配优先，然后按编辑距离
        candidates.sort_by(|a, b| {
            // 前缀匹配优先（true > false，所以反过来比较）
            match (a.2, b.2) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.3.cmp(&b.3), // 同类型按编辑距离排序
            }
        });

        // 提取简短释义并返回
        let results: Vec<WordSuggestion> = candidates
            .into_iter()
            .take(limit)
            .map(|(word, content, _, _)| {
                let brief = extract_brief(&content);
                WordSuggestion { word, brief }
            })
            .collect();

        Ok(results)
    }
}

/// 获取词典数据库路径
fn get_db_path(app: &AppHandle) -> PathBuf {
    // 开发模式：从 src-tauri/resources 读取
    // 生产模式：从 resource_dir 读取
    let resource_dir = app.path().resource_dir().expect("Failed to get resource dir");
    let prod_path = resource_dir.join("dict.db");

    if prod_path.exists() {
        return prod_path;
    }

    // 开发模式 fallback
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("dict.db");
    dev_path
}

/// 初始化词典
pub fn init_dictionary(app: &AppHandle) -> Result<(), String> {
    let db_path = get_db_path(app);

    if !db_path.exists() {
        return Err(format!("Dictionary database not found: {:?}", db_path));
    }

    let state = app.state::<DictionaryState>();
    state
        .init(db_path)
        .map_err(|e| format!("Failed to open dictionary: {}", e))
}

/// Tauri command: 查询单词
#[tauri::command]
pub fn lookup_word(word: String, state: tauri::State<DictionaryState>) -> Result<Option<WordEntry>, String> {
    state
        .lookup(&word)
        .map_err(|e| format!("Lookup failed: {}", e))
}

/// Tauri command: 搜索单词（模糊匹配）
#[tauri::command]
pub fn search_words(query: String, state: tauri::State<DictionaryState>) -> Result<Vec<WordSuggestion>, String> {
    if query.len() < 2 {
        return Ok(vec![]);
    }
    state
        .search(&query, 8)
        .map_err(|e| format!("Search failed: {}", e))
}

/// 从 content JSON 中提取简短释义
fn extract_brief(content: &str) -> String {
    // 尝试解析 JSON 并提取第一个翻译
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(trans) = json
            .get("content")
            .and_then(|c| c.get("word"))
            .and_then(|w| w.get("content"))
            .and_then(|c| c.get("trans"))
            .and_then(|t| t.as_array())
        {
            let mut parts: Vec<String> = vec![];
            for t in trans.iter().take(2) {
                let pos = t.get("pos").and_then(|p| p.as_str()).unwrap_or("");
                let tran = t.get("tranCn").and_then(|t| t.as_str()).unwrap_or("");
                if !tran.is_empty() {
                    if !pos.is_empty() {
                        parts.push(format!("{} {}", pos, tran));
                    } else {
                        parts.push(tran.to_string());
                    }
                }
            }
            if !parts.is_empty() {
                return parts.join("; ");
            }
        }
    }
    String::new()
}
