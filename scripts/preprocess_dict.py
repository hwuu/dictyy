#!/usr/bin/env python3
"""
词典数据预处理脚本

功能：
1. 从 kajweb/dict 源提取 ZIP 文件
2. 解析 JSON 数据，保留全部字段
3. 合并去重单词
4. 导入 DictionaryByGPT4 数据作为补充
5. 输出 SQLite 数据库
"""

import json
import sqlite3
import zipfile
from pathlib import Path
from typing import Dict, List, Set

# GPT4 词典路径
GPT4_DICT_PATH = Path(r"C:\Users\hwuu\dev\github\Ceelog\DictionaryByGPT4\gptwords.json")


# 词典源文件映射
DICT_FILES = {
    "CET4": "1521164643060_CET4_3.zip",
    "CET6": "1521164633851_CET6_3.zip",
    "TOEFL": ["1521164640451_TOEFL_2.zip", "1521164667985_TOEFL_3.zip"],
    "IELTS": ["1521164657744_IELTS_2.zip", "1521164666922_IELTS_3.zip"],
    "GRE": ["1521164637271_GRE_2.zip", "1521164677706_GRE_3.zip"],
    "GMAT": ["1521164629611_GMATluan_2.zip", "1521164672691_GMAT_3.zip"],
}


def create_database(db_path: Path) -> sqlite3.Connection:
    """创建 SQLite 数据库和表结构"""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # 创建单词表 (kajweb/dict 数据)
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS words (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word TEXT NOT NULL UNIQUE,
            phonetic_us TEXT,
            phonetic_uk TEXT,
            content TEXT NOT NULL
        )
    """)

    # 创建词典来源表
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS word_sources (
            word_id INTEGER NOT NULL,
            source TEXT NOT NULL,
            FOREIGN KEY (word_id) REFERENCES words(id),
            PRIMARY KEY (word_id, source)
        )
    """)

    # 创建 GPT4 词典表 (DictionaryByGPT4 数据)
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS gpt4_words (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL
        )
    """)

    # 创建索引
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_words_word ON words(word)")
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_gpt4_words_word ON gpt4_words(word)")

    conn.commit()
    return conn


def extract_json_from_zip(zip_path: Path) -> List[Dict]:
    """从 ZIP 文件中提取 JSON 数据"""
    words = []

    with zipfile.ZipFile(zip_path, 'r') as zf:
        for name in zf.namelist():
            if name.endswith('.json'):
                with zf.open(name) as f:
                    content = f.read().decode('utf-8')
                    # 每行一个 JSON 对象
                    for line in content.strip().split('\n'):
                        if line.strip():
                            try:
                                word_data = json.loads(line)
                                words.append(word_data)
                            except json.JSONDecodeError as e:
                                print(f"JSON 解析错误: {e}")

    return words


def extract_word_info(word_data: Dict) -> Dict:
    """从原始数据中提取单词信息"""
    head_word = word_data.get("headWord", "")

    # 提取音标
    content = word_data.get("content", {})
    word_content = content.get("word", {}).get("content", {})

    phonetic_us = word_content.get("usphone", "")
    phonetic_uk = word_content.get("ukphone", "")

    return {
        "word": head_word,
        "phonetic_us": phonetic_us,
        "phonetic_uk": phonetic_uk,
        "content": json.dumps(word_data, ensure_ascii=False)  # 保留完整原始数据
    }


def process_dict_source(source_path: Path, source_name: str, conn: sqlite3.Connection) -> int:
    """处理单个词典源文件"""
    words = extract_json_from_zip(source_path)
    cursor = conn.cursor()
    added_count = 0

    for word_data in words:
        info = extract_word_info(word_data)
        word = info["word"]

        if not word:
            continue

        # 尝试插入单词（如果已存在则忽略）
        try:
            cursor.execute("""
                INSERT INTO words (word, phonetic_us, phonetic_uk, content)
                VALUES (?, ?, ?, ?)
            """, (word, info["phonetic_us"], info["phonetic_uk"], info["content"]))
            word_id = cursor.lastrowid
            added_count += 1
        except sqlite3.IntegrityError:
            # 单词已存在，获取其 ID
            cursor.execute("SELECT id FROM words WHERE word = ?", (word,))
            word_id = cursor.fetchone()[0]

        # 添加词典来源关联
        try:
            cursor.execute("""
                INSERT INTO word_sources (word_id, source)
                VALUES (?, ?)
            """, (word_id, source_name))
        except sqlite3.IntegrityError:
            # 来源关联已存在
            pass

    conn.commit()
    return added_count


def process_gpt4_dict(gpt4_path: Path, conn: sqlite3.Connection) -> int:
    """处理 GPT4 词典数据"""
    if not gpt4_path.exists():
        print(f"警告: GPT4 词典文件不存在 - {gpt4_path}")
        return 0

    cursor = conn.cursor()
    added_count = 0

    with open(gpt4_path, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            try:
                data = json.loads(line)
                word = data.get("word", "").strip()
                content = data.get("content", "")

                if not word:
                    continue

                cursor.execute("""
                    INSERT OR IGNORE INTO gpt4_words (word, content)
                    VALUES (?, ?)
                """, (word, content))

                if cursor.rowcount > 0:
                    added_count += 1

            except json.JSONDecodeError as e:
                print(f"JSON 解析错误: {e}")

    conn.commit()
    return added_count


def main():
    # 路径配置
    dict_source_dir = Path(r"C:\Users\hwuu\dev\github\kajweb\dict\book")
    output_dir = Path(r"C:\Users\hwuu\dev\hwuu\dictyy\src-tauri\resources")
    output_dir.mkdir(parents=True, exist_ok=True)

    db_path = output_dir / "dict.db"

    # 如果数据库已存在，删除重建
    if db_path.exists():
        db_path.unlink()

    print(f"创建数据库: {db_path}")
    conn = create_database(db_path)

    total_words = 0

    # 处理每个词典源
    for source_name, files in DICT_FILES.items():
        if isinstance(files, str):
            files = [files]

        for file_name in files:
            zip_path = dict_source_dir / file_name
            if not zip_path.exists():
                print(f"警告: 文件不存在 - {zip_path}")
                continue

            print(f"处理: {source_name} - {file_name}")
            added = process_dict_source(zip_path, source_name, conn)
            print(f"  新增 {added} 个单词")
            total_words += added

    # 处理 GPT4 词典
    print(f"\n处理: DictionaryByGPT4 - gptwords.json")
    gpt4_added = process_gpt4_dict(GPT4_DICT_PATH, conn)
    print(f"  新增 {gpt4_added} 个单词")

    # 统计信息
    cursor = conn.cursor()
    cursor.execute("SELECT COUNT(*) FROM words")
    unique_words = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM gpt4_words")
    gpt4_words = cursor.fetchone()[0]

    cursor.execute("SELECT source, COUNT(*) FROM word_sources GROUP BY source")
    source_stats = cursor.fetchall()

    # 统计两表可 join 的单词数
    cursor.execute("""
        SELECT COUNT(*) FROM words w
        INNER JOIN gpt4_words g ON LOWER(w.word) = LOWER(g.word)
    """)
    joinable_words = cursor.fetchone()[0]

    print(f"\n=== 统计信息 ===")
    print(f"words 表单词数: {unique_words}")
    print(f"gpt4_words 表单词数: {gpt4_words}")
    print(f"可 JOIN 单词数: {joinable_words}")
    print(f"\n各词典来源:")
    for source, count in source_stats:
        print(f"  {source}: {count} 词")

    conn.close()
    print(f"\n数据库已保存到: {db_path}")


if __name__ == "__main__":
    main()
