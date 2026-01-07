#!/usr/bin/env python3
"""
生成 word_abstracts 表
将所有词典的摘要信息预处理到一个表中，供气泡快速查询
"""

import sqlite3
import json
import re
from pathlib import Path


def extract_main_def(content_str: str) -> tuple[str, str, str]:
    """从主词典提取音标和释义"""
    try:
        data = json.loads(content_str)
        content = data.get("content", {})
        word_data = content.get("word", {}).get("content", {})

        # 提取音标
        phonetic = ""
        if "usphone" in word_data:
            phonetic = word_data["usphone"]
        elif "ukphone" in word_data:
            phonetic = word_data["ukphone"]

        # 提取释义
        trans_list = word_data.get("trans", [])
        defs = []
        for t in trans_list[:2]:  # 最多取2条
            pos = t.get("pos", "")
            tran = t.get("tranCn", "")
            if pos and tran:
                defs.append(f"{pos} {tran}")
            elif tran:
                defs.append(tran)

        return phonetic, "; ".join(defs), ""
    except:
        return "", "", ""


def extract_collins_def(content_str: str) -> tuple[str, str]:
    """从柯林斯提取音标和释义"""
    try:
        data = json.loads(content_str)
        if not data:
            return "", ""

        # 提取音标
        phonetic = data.get("phonetic_uk", "") or data.get("phonetic_us", "")

        # 提取释义
        definitions = data.get("definitions", [])
        defs = []
        for d in definitions[:2]:  # 最多取2条
            pos = d.get("pos", "")
            cn = d.get("cn", "")
            if cn:
                # 清理释义中的多余空格
                cn = re.sub(r'\s+', ' ', cn).strip()
                if pos:
                    defs.append(f"{pos} {cn}")
                else:
                    defs.append(cn)

        return phonetic, "; ".join(defs)
    except:
        return "", ""


def extract_etyma_def(content_str: str) -> str:
    """从词根词缀提取词源"""
    try:
        data = json.loads(content_str)
        if not data:
            return ""

        etymology = data.get("etymology", "")
        root = data.get("root", "")

        if etymology:
            return etymology
        elif root:
            return root
        return ""
    except:
        return ""


def extract_gpt4_def(content_str: str) -> str:
    """从 GPT4 提取简短释义"""
    try:
        # GPT4 内容是 Markdown，提取第一段有意义的内容
        lines = content_str.split("\n")
        for line in lines:
            line = line.strip()
            # 跳过标题和空行
            if not line or line.startswith("#"):
                continue
            # 跳过太短的行
            if len(line) < 10:
                continue
            # 返回第一段内容（截断到100字符）
            return line[:100] + ("..." if len(line) > 100 else "")
        return ""
    except:
        return ""


def generate_abstracts(db_path: str):
    """生成所有词的 abstract"""
    print(f"Opening database: {db_path}", flush=True)
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # 创建 word_abstracts 表
    print("Creating word_abstracts table...", flush=True)
    cursor.execute("DROP TABLE IF EXISTS word_abstracts")
    cursor.execute("""
        CREATE TABLE word_abstracts (
            word TEXT PRIMARY KEY,
            phonetic TEXT,
            main_def TEXT,
            collins_def TEXT,
            etyma_def TEXT,
            gpt4_def TEXT
        )
    """)
    cursor.execute("CREATE INDEX idx_abstracts_word ON word_abstracts(word)")
    print("Table created.", flush=True)

    # 批量加载所有数据到内存（避免每个词都查询数据库）
    print("Loading all data into memory...", flush=True)

    # 加载主词典
    print("  - Loading words table...", flush=True)
    words_data = {}
    cursor.execute("SELECT word, phonetic_us, phonetic_uk, content FROM words")
    for row in cursor.fetchall():
        word_lower = row[0].lower()
        words_data[word_lower] = (row[1], row[2], row[3])
    print(f"    Loaded {len(words_data)} entries", flush=True)

    # 加载柯林斯
    print("  - Loading collins_words table...", flush=True)
    collins_data = {}
    cursor.execute("SELECT word, content FROM collins_words WHERE is_link = 0")
    for row in cursor.fetchall():
        word_lower = row[0].lower()
        collins_data[word_lower] = row[1]
    print(f"    Loaded {len(collins_data)} entries", flush=True)

    # 加载词根词缀
    print("  - Loading etyma_words table...", flush=True)
    etyma_data = {}
    cursor.execute("SELECT word, content FROM etyma_words WHERE is_link = 0")
    for row in cursor.fetchall():
        word_lower = row[0].lower()
        etyma_data[word_lower] = row[1]
    print(f"    Loaded {len(etyma_data)} entries", flush=True)

    # 加载 GPT4
    print("  - Loading gpt4_words table...", flush=True)
    gpt4_data = {}
    cursor.execute("SELECT word, content FROM gpt4_words")
    for row in cursor.fetchall():
        word_lower = row[0].lower()
        gpt4_data[word_lower] = row[1]
    print(f"    Loaded {len(gpt4_data)} entries", flush=True)

    # 收集所有词
    print("Collecting all unique words...", flush=True)
    all_words = set()
    all_words.update(words_data.keys())
    all_words.update(collins_data.keys())
    all_words.update(etyma_data.keys())
    all_words.update(gpt4_data.keys())
    print(f"Total unique words: {len(all_words)}", flush=True)

    print("Generating abstracts...", flush=True)

    # 为每个词生成 abstract（从内存查找，不再查数据库）
    count = 0
    total = len(all_words)
    for word in all_words:
        phonetic = ""
        main_def = ""
        collins_def = ""
        etyma_def = ""
        gpt4_def = ""

        # 从内存查询主词典
        if word in words_data:
            phonetic_us, phonetic_uk, content = words_data[word]
            phonetic = phonetic_us or phonetic_uk or ""
            _, main_def, _ = extract_main_def(content)

        # 从内存查询柯林斯
        if word in collins_data:
            p, collins_def = extract_collins_def(collins_data[word])
            if not phonetic and p:
                phonetic = p

        # 从内存查询词根词缀
        if word in etyma_data:
            etyma_def = extract_etyma_def(etyma_data[word])

        # 从内存查询 GPT4
        if word in gpt4_data:
            gpt4_def = extract_gpt4_def(gpt4_data[word])

        # 插入 abstract
        cursor.execute("""
            INSERT OR REPLACE INTO word_abstracts
            (word, phonetic, main_def, collins_def, etyma_def, gpt4_def)
            VALUES (?, ?, ?, ?, ?, ?)
        """, (word, phonetic, main_def, collins_def, etyma_def, gpt4_def))

        count += 1
        if count % 10000 == 0:
            pct = count * 100 // total
            print(f"  [{pct:3d}%] Processed {count}/{total} words...", flush=True)
            conn.commit()

    conn.commit()
    print(f"  [100%] Processed {count}/{total} words.", flush=True)

    # 统计结果
    cursor.execute("SELECT COUNT(*) FROM word_abstracts")
    total = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM word_abstracts WHERE main_def != ''")
    with_main = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM word_abstracts WHERE collins_def != ''")
    with_collins = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM word_abstracts WHERE etyma_def != ''")
    with_etyma = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM word_abstracts WHERE gpt4_def != ''")
    with_gpt4 = cursor.fetchone()[0]

    print(f"\n=== Summary ===")
    print(f"Total abstracts: {total}")
    print(f"With main_def: {with_main}")
    print(f"With collins_def: {with_collins}")
    print(f"With etyma_def: {with_etyma}")
    print(f"With gpt4_def: {with_gpt4}")

    conn.close()


if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1:
        db_path = sys.argv[1]
    else:
        # 默认路径
        db_path = Path(__file__).parent.parent / "src-tauri" / "target" / "debug" / "resources" / "dict.db"

    print(f"Processing: {db_path}")
    generate_abstracts(str(db_path))
    print("Done!")
