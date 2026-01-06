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

/// MDX 词典查询结果
#[derive(Debug, Serialize, Deserialize)]
pub struct MdxEntry {
    pub word: String,
    pub content: String,  // JSON 格式的内容
    pub is_link: bool,
    pub link_target: Option<String>,
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

    /// 搜索单词（模糊匹配）- 查询所有词典
    pub fn search(&self, query: &str, limit: usize) -> SqliteResult<Vec<WordSuggestion>> {
        let lock = self.conn.lock().unwrap();
        let conn = match lock.as_ref() {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let query_lower = query.to_lowercase();
        let pattern = format!("{}%", query_lower);

        // (word, brief, is_prefix, distance) - 收集所有词典的结果
        let mut candidates: Vec<(String, String, bool, usize)> = Vec::new();
        let mut seen_words: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 1. 查询主词典 (words)
        {
            let mut stmt = conn.prepare(
                "SELECT word, content FROM words
                 WHERE LOWER(word) LIKE ?1
                 ORDER BY LENGTH(word) ASC
                 LIMIT 30"
            )?;

            let results: Vec<(String, String)> = stmt
                .query_map([&pattern], |row| {
                    let word: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    Ok((word, content))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (word, content) in results {
                let word_lower = word.to_lowercase();
                if !seen_words.contains(&word_lower) {
                    let distance = levenshtein(&query_lower, &word_lower);
                    let brief = extract_brief(&content);
                    candidates.push((word.clone(), brief, true, distance));
                    seen_words.insert(word_lower);
                }
            }
        }

        // 2. 查询柯林斯词典 (collins_words)
        {
            let mut stmt = conn.prepare(
                "SELECT word, content FROM collins_words
                 WHERE LOWER(word) LIKE ?1 AND is_link = 0
                 ORDER BY LENGTH(word) ASC
                 LIMIT 20"
            )?;

            let results: Vec<(String, String)> = stmt
                .query_map([&pattern], |row| {
                    let word: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    Ok((word, content))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (word, content) in results {
                let word_lower = word.to_lowercase();
                if !seen_words.contains(&word_lower) {
                    let distance = levenshtein(&query_lower, &word_lower);
                    let brief = extract_collins_brief(&content);
                    candidates.push((word.clone(), brief, true, distance));
                    seen_words.insert(word_lower);
                }
            }
        }

        // 3. 查询词根词缀词典 (etyma_words)
        {
            let mut stmt = conn.prepare(
                "SELECT word, content FROM etyma_words
                 WHERE LOWER(word) LIKE ?1 AND is_link = 0
                 ORDER BY LENGTH(word) ASC
                 LIMIT 10"
            )?;

            let results: Vec<(String, String)> = stmt
                .query_map([&pattern], |row| {
                    let word: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    Ok((word, content))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (word, content) in results {
                let word_lower = word.to_lowercase();
                if !seen_words.contains(&word_lower) {
                    let distance = levenshtein(&query_lower, &word_lower);
                    let brief = extract_etyma_brief(&content);
                    candidates.push((word.clone(), brief, true, distance));
                    seen_words.insert(word_lower);
                }
            }
        }

        // 4. 如果前缀匹配结果不足，从主词典补充编辑距离相近的词
        if candidates.len() < limit {
            let mut stmt2 = conn.prepare(
                "SELECT word, content FROM words
                 WHERE LOWER(word) NOT LIKE ?1
                 AND LENGTH(word) BETWEEN ?2 AND ?3
                 LIMIT 50"
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
                .filter_map(|(word, content)| {
                    let word_lower = word.to_lowercase();
                    if seen_words.contains(&word_lower) {
                        return None;
                    }
                    let distance = levenshtein(&query_lower, &word_lower);
                    if distance <= 2 {
                        let brief = extract_brief(&content);
                        Some((word, brief, false, distance))
                    } else {
                        None
                    }
                })
                .collect();

            candidates.extend(additional);
        }

        // 排序：前缀匹配优先，然后按编辑距离
        candidates.sort_by(|a, b| {
            match (a.2, b.2) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.3.cmp(&b.3),
            }
        });

        // 返回结果
        let results: Vec<WordSuggestion> = candidates
            .into_iter()
            .take(limit)
            .map(|(word, brief, _, _)| WordSuggestion { word, brief })
            .collect();

        Ok(results)
    }

    /// 查询柯林斯词典（支持 link 递归解析）
    pub fn lookup_collins(&self, word: &str) -> SqliteResult<Option<MdxEntry>> {
        self.lookup_mdx_table("collins_words", word, 5)
    }

    /// 查询词根词缀词典（支持 link 递归解析）
    pub fn lookup_etyma(&self, word: &str) -> SqliteResult<Option<MdxEntry>> {
        self.lookup_mdx_table("etyma_words", word, 5)
    }

    /// 通用 MDX 表查询（支持 link 递归解析）
    fn lookup_mdx_table(&self, table: &str, word: &str, max_depth: u32) -> SqliteResult<Option<MdxEntry>> {
        let lock = self.conn.lock().unwrap();
        let conn = match lock.as_ref() {
            Some(c) => c,
            None => return Ok(None),
        };

        let mut current_word = word.to_string();
        let mut depth = 0;

        loop {
            let query = format!(
                "SELECT word, content, is_link, link_target FROM {} WHERE LOWER(word) = LOWER(?1)",
                table
            );
            let mut stmt = conn.prepare(&query)?;

            let result = stmt.query_row([&current_word], |row| {
                Ok(MdxEntry {
                    word: row.get(0)?,
                    content: row.get(1)?,
                    is_link: row.get::<_, i32>(2)? != 0,
                    link_target: row.get(3)?,
                })
            });

            match result {
                Ok(entry) => {
                    if entry.is_link && depth < max_depth {
                        if let Some(ref target) = entry.link_target {
                            current_word = target.clone();
                            depth += 1;
                            continue;
                        }
                    }
                    return Ok(Some(entry));
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(e),
            }
        }
    }

    /// 查询 GPT4 词典
    pub fn lookup_gpt4(&self, word: &str) -> SqliteResult<Option<String>> {
        let lock = self.conn.lock().unwrap();
        let conn = match lock.as_ref() {
            Some(c) => c,
            None => return Ok(None),
        };

        let mut stmt = conn.prepare(
            "SELECT content FROM gpt4_words WHERE LOWER(word) = LOWER(?1)"
        )?;

        match stmt.query_row([word], |row| row.get(0)) {
            Ok(content) => Ok(Some(content)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// 获取词典数据库路径
fn get_db_path(app: &AppHandle) -> Option<PathBuf> {
    // 生产模式：从 resource_dir 读取
    if let Ok(resource_dir) = app.path().resource_dir() {
        // 尝试多个可能的路径
        let candidates = [
            resource_dir.join("dict.db"),
            resource_dir.join("resources").join("dict.db"),
        ];

        for path in &candidates {
            crate::debug_log(&format!("[Dictionary] Checking path: {:?}, exists: {}", path, path.exists()));
            if path.exists() {
                return Some(path.clone());
            }
        }
    }

    // 开发模式：检查当前工作目录
    let dev_candidates = [
        std::env::current_dir().ok().map(|p| p.join("src-tauri").join("resources").join("dict.db")),
        std::env::current_dir().ok().map(|p| p.join("resources").join("dict.db")),
    ];

    for path_opt in &dev_candidates {
        if let Some(path) = path_opt {
            crate::debug_log(&format!("[Dictionary] Checking dev path: {:?}, exists: {}", path, path.exists()));
            if path.exists() {
                return Some(path.clone());
            }
        }
    }

    None
}

/// 初始化词典
pub fn init_dictionary(app: &AppHandle) -> Result<(), String> {
    let db_path = get_db_path(app).ok_or("Dictionary database not found in any expected location")?;
    crate::debug_log(&format!("[Dictionary] Using db path: {:?}", db_path));

    let state = app.state::<DictionaryState>();
    state
        .init(db_path.clone())
        .map_err(|e| {
            let err = format!("Failed to open dictionary: {}", e);
            crate::debug_log(&format!("[Dictionary] {}", err));
            err
        })?;

    crate::debug_log("[Dictionary] Successfully initialized");
    Ok(())
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

/// Tauri command: 查询柯林斯词典
#[tauri::command]
pub fn lookup_collins(word: String, state: tauri::State<DictionaryState>) -> Result<Option<MdxEntry>, String> {
    state
        .lookup_collins(&word)
        .map_err(|e| format!("Collins lookup failed: {}", e))
}

/// Tauri command: 查询词根词缀词典
#[tauri::command]
pub fn lookup_etyma(word: String, state: tauri::State<DictionaryState>) -> Result<Option<MdxEntry>, String> {
    state
        .lookup_etyma(&word)
        .map_err(|e| format!("Etyma lookup failed: {}", e))
}

/// Tauri command: 查询 GPT4 词典
#[tauri::command]
pub fn lookup_gpt4(word: String, state: tauri::State<DictionaryState>) -> Result<Option<String>, String> {
    state
        .lookup_gpt4(&word)
        .map_err(|e| format!("GPT4 lookup failed: {}", e))
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

/// 从柯林斯词典 JSON 中提取简短释义
fn extract_collins_brief(content: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(definitions) = json.get("definitions").and_then(|d| d.as_array()) {
            let mut parts: Vec<String> = vec![];
            for def in definitions.iter().take(2) {
                let pos = def.get("pos").and_then(|p| p.as_str()).unwrap_or("");
                let cn = def.get("cn").and_then(|c| c.as_str()).unwrap_or("");
                if !cn.is_empty() {
                    if !pos.is_empty() {
                        parts.push(format!("{} {}", pos, cn));
                    } else {
                        parts.push(cn.to_string());
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

/// 从词根词缀词典 JSON 中提取简短释义
fn extract_etyma_brief(content: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        let pos = json.get("pos").and_then(|p| p.as_str()).unwrap_or("");
        let meaning = json.get("meaning").and_then(|m| m.as_str()).unwrap_or("");
        if !meaning.is_empty() {
            if !pos.is_empty() {
                return format!("{} {}", pos, meaning);
            } else {
                return meaning.to_string();
            }
        }
    }
    String::new()
}
