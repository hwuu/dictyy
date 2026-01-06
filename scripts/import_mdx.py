#!/usr/bin/env python3
"""
MDX 词典导入脚本

功能：
1. 解析柯林斯英汉双解词典 MDX → JSON → collins_words 表
2. 解析英语词根词缀词频 MDX → JSON → etyma_words 表
3. 处理 @@@LINK 类型词条
"""

import json
import re
import sqlite3
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from html.parser import HTMLParser
from mdict_utils.reader import MDX


# ============================================================================
# 柯林斯词典解析器
# ============================================================================

class CollinsHTMLParser(HTMLParser):
    """解析柯林斯词典 HTML 并提取结构化数据"""

    def __init__(self):
        super().__init__()
        self.result = {
            "word": "",
            "phonetic_uk": [],  # 改为数组，支持多发音
            "phonetic_us": [],
            "frequency": 0,
            "forms": [],
            "definitions": []
        }

        # 标签栈，用于追踪嵌套
        self._tag_stack: List[Tuple[str, str]] = []  # (tag, class)

        # 状态跟踪
        self._in_word_key = False
        self._in_pron_uk = False
        self._in_pron_us = False
        self._pron_uk_depth = 0  # 跟踪进入 pron_uk 时的栈深度
        self._pron_us_depth = 0
        self._current_pron_uk = ""  # 当前正在收集的音标
        self._current_pron_us = ""
        self._in_form_inflected = False
        self._in_orth = False
        self._orth_depth = 0
        self._in_example = False
        self._in_caption = False
        self._in_def_cn = False
        self._def_cn_depth = 0
        self._in_chinese_text = False
        self._chinese_text_depth = 0
        self._in_num = False
        self._in_st = False
        self._in_li = False
        self._in_li_p = False
        self._li_p_count = 0  # 跟踪 li 内的 p 标签数量

        # 当前释义
        self._current_def = None
        self._current_example_en = ""
        self._current_example_cn = ""
        self._collecting_en_def = False

        # 词频计数
        self._frequency_count = 0

    def handle_starttag(self, tag: str, attrs: List[Tuple[str, str]]):
        attrs_dict = dict(attrs)
        current_class = attrs_dict.get("class", "")
        self._tag_stack.append((tag, current_class))
        stack_depth = len(self._tag_stack)

        if tag == "span":
            if "word_key" in current_class:
                self._in_word_key = True
            elif "pron type_uk" in current_class:
                self._in_pron_uk = True
                self._pron_uk_depth = stack_depth
                self._current_pron_uk = ""
            elif "pron type_us" in current_class:
                self._in_pron_us = True
                self._pron_us_depth = stack_depth
                self._current_pron_us = ""
            elif current_class == "num":
                self._in_num = True
            elif current_class == "st":
                self._in_st = True
            elif "level" in current_class and "roundRed" in current_class:
                self._frequency_count += 1
            elif "def_cn" in current_class and "cn_before" in current_class:
                # 只收集 cn_before，忽略 cn_after（内容重复）
                self._in_def_cn = True
                self._def_cn_depth = stack_depth
            elif "chinese-text" in current_class:
                self._in_chinese_text = True
                self._chinese_text_depth = stack_depth
            # 忽略 icon-speak 等其他 span

        elif tag == "div":
            if "form_inflected" in current_class:
                self._in_form_inflected = True
            elif "collins_en_cn example" in current_class:
                self._in_example = True
                self._current_def = {
                    "num": "",
                    "pos": "",
                    "cn": "",
                    "en": "",
                    "examples": [],
                    "synonyms": []
                }
            elif "caption" in current_class and "hide_cn" in current_class:
                self._in_caption = True
                self._collecting_en_def = True

        elif tag == "a":
            if self._in_form_inflected and "orth" in current_class:
                self._in_orth = True
                self._orth_depth = stack_depth

        # 处理非 <a> 的 orth (如 records 没有发音链接)
        if tag == "span" and self._in_form_inflected and "orth" in current_class:
            self._in_orth = True
            self._orth_depth = stack_depth

        elif tag == "li":
            if self._in_example:
                self._in_li = True
                self._li_p_count = 0
                self._current_example_en = ""
                self._current_example_cn = ""

        elif tag == "p":
            if self._in_li:
                self._in_li_p = True
                self._li_p_count += 1

    def handle_endtag(self, tag: str):
        if not self._tag_stack:
            return

        stack_depth = len(self._tag_stack)

        if tag == "span":
            # 检查是否关闭特定状态的 span
            if self._in_pron_uk and stack_depth == self._pron_uk_depth:
                # 保存当前音标
                pron = self._current_pron_uk.strip()
                if pron:
                    self.result["phonetic_uk"].append(pron)
                self._in_pron_uk = False
                self._current_pron_uk = ""
            elif self._in_pron_us and stack_depth == self._pron_us_depth:
                pron = self._current_pron_us.strip()
                if pron:
                    self.result["phonetic_us"].append(pron)
                self._in_pron_us = False
                self._current_pron_us = ""
            elif self._in_word_key:
                self._in_word_key = False
            elif self._in_num:
                self._in_num = False
            elif self._in_st:
                self._in_st = False
            elif self._in_chinese_text and stack_depth == self._chinese_text_depth:
                self._in_chinese_text = False
            elif self._in_def_cn and stack_depth == self._def_cn_depth:
                self._in_def_cn = False
            elif self._in_orth and stack_depth == self._orth_depth:
                self._in_orth = False

        elif tag == "div":
            if self._in_example:
                # 检查是否是 collins_en_cn example div 的关闭
                # 通过检查栈顶来判断
                if self._tag_stack and "collins_en_cn example" in self._tag_stack[-1][1]:
                    if self._current_def and (self._current_def["en"] or self._current_def["cn"]):
                        self.result["definitions"].append(self._current_def)
                    self._in_example = False
                    self._current_def = None
            if self._in_caption and self._tag_stack and "caption" in self._tag_stack[-1][1]:
                self._in_caption = False
                self._collecting_en_def = False
            if self._in_form_inflected and self._tag_stack and "form_inflected" in self._tag_stack[-1][1]:
                self._in_form_inflected = False

        elif tag == "a":
            if self._in_orth and stack_depth == self._orth_depth:
                self._in_orth = False

        elif tag == "li":
            if self._in_li and self._current_def:
                en = self._current_example_en.strip()
                cn = self._current_example_cn.strip()
                if en or cn:
                    self._current_def["examples"].append({"en": en, "cn": cn})
            self._in_li = False
            self._in_li_p = False

        elif tag == "p":
            self._in_li_p = False

        # 出栈
        if self._tag_stack:
            self._tag_stack.pop()

    def handle_data(self, data: str):
        text = data.strip()
        if not text:
            return

        if self._in_word_key:
            self.result["word"] = text
        elif self._in_pron_uk:
            self._current_pron_uk += data  # 保留原始空格
        elif self._in_pron_us:
            self._current_pron_us += data
        elif self._in_orth:
            # 只添加有效的词形（字母、连字符、撇号）
            if re.match(r'^[a-zA-Z\-\' ]+$', text):
                self.result["forms"].append(text)
        elif self._in_num and self._current_def:
            self._current_def["num"] = text
        elif self._in_st and self._current_def:
            self._current_def["pos"] = text.strip()
        elif self._in_def_cn and self._current_def:
            # 收集中文释义（包括英文词和中文，如 "rural route的缩写"）
            if self._current_def["cn"]:
                self._current_def["cn"] += text
            else:
                self._current_def["cn"] = text
        elif self._in_li:
            # 例句：第一个 p 是英文，第二个 p 是中文
            if self._li_p_count == 1:
                self._current_example_en += data + " "
            elif self._li_p_count >= 2:
                # 收集所有文本（包括数字和标点）
                self._current_example_cn += text
        elif self._collecting_en_def and self._current_def and not self._in_def_cn and not self._in_num and not self._in_st:
            # 收集英文释义
            self._current_def["en"] += data + " "

    def get_result(self) -> Dict:
        self.result["frequency"] = self._frequency_count
        # 合并多个音标，用 " / " 分隔
        uk_prons = self.result["phonetic_uk"]
        us_prons = self.result["phonetic_us"]
        self.result["phonetic_uk"] = " / ".join(uk_prons) if uk_prons else ""
        self.result["phonetic_us"] = " / ".join(us_prons) if us_prons else ""
        # 去重 forms
        seen = set()
        unique_forms = []
        for f in self.result["forms"]:
            if f not in seen:
                seen.add(f)
                unique_forms.append(f)
        self.result["forms"] = unique_forms
        return self.result


def parse_collins_html(html: str) -> Dict:
    """解析柯林斯 HTML 为 JSON 结构"""
    parser = CollinsHTMLParser()
    try:
        parser.feed(html)
        return parser.get_result()
    except Exception as e:
        return {"error": str(e), "raw_html": html[:500]}


# ============================================================================
# 词根词缀词典解析器
# ============================================================================

def parse_etyma_html(html: str) -> Dict:
    """解析词根词缀 HTML 为 JSON 结构

    结构示例：
    abandon<font color=orange> v </font>抛弃,放弃<font color=indianred>(a 不+ban+don 给予=...)</font>
    <font color=RED>  ★★★ </font> <font color=blue>  7222 </font>
    """
    result = {
        "word": "",
        "pos": "",
        "meaning": "",
        "etymology": "",
        "frequency": 0,
        "stars": 0,
        "root": "",
        "related": []
    }

    # 移除 HTML 标签但保留结构信息
    lines = html.split("<br>")
    if not lines:
        lines = html.split("<BR>")

    first_line = lines[0] if lines else html

    # 提取主单词（第一个非标签文本）
    word_match = re.match(r'^([a-zA-Z][a-zA-Z\-\' ]*)', first_line)
    if word_match:
        result["word"] = word_match.group(1).strip()

    # 提取词性 <font color=orange> v </font>
    pos_match = re.search(r'<font color=orange>\s*([a-z./]+)\s*</font>', first_line, re.I)
    if pos_match:
        result["pos"] = pos_match.group(1).strip()

    # 提取释义（词性后到词源前的文本）
    # 移除标签后提取
    clean_line = re.sub(r'<[^>]+>', '', first_line)
    # 释义在词性和括号之间
    meaning_match = re.search(r'[a-z./]\s+([^(★\d]+)', clean_line, re.I)
    if meaning_match:
        result["meaning"] = meaning_match.group(1).strip().strip(',')

    # 提取词源解释 <font color=indianred>(...)</font>
    etymology_match = re.search(r'<font color=indianred>\(([^)]+)\)</font>', first_line, re.I)
    if etymology_match:
        result["etymology"] = etymology_match.group(1).strip()

    # 提取星级 ★ 数量
    stars = first_line.count('★')
    result["stars"] = stars

    # 提取词频 <font color=blue>  7222 </font>
    freq_match = re.search(r'<font color=blue>\s*(\d+)\s*</font>', first_line, re.I)
    if freq_match:
        result["frequency"] = int(freq_match.group(1))
    else:
        # 也可能是直接跟在星号后面的数字
        freq_match2 = re.search(r'★+\s*(\d+)', first_line)
        if freq_match2:
            result["frequency"] = int(freq_match2.group(1))

    # 提取词根说明 <font color=teal>...</font>
    root_match = re.search(r'<font color=teal>([^<]+)</font>', html, re.I)
    if root_match:
        result["root"] = root_match.group(1).strip()

    # 提取相关单词（后续行中的单词）
    for line in lines[1:]:
        line = line.strip()
        if not line or line.startswith('<DIV') or line.startswith('<font color=teal'):
            continue

        # 提取单词
        related_word_match = re.match(r'^([a-zA-Z][a-zA-Z\-\' ]*)', line)
        if related_word_match:
            related_word = related_word_match.group(1).strip()
            if related_word and related_word != result["word"]:
                # 提取该单词的简要信息（去掉开头的单词）
                clean_related = re.sub(r'<[^>]+>', '', line)
                # 移除开头的单词名称
                brief = clean_related[len(related_word):].strip()
                result["related"].append({
                    "word": related_word,
                    "brief": brief[:100] if len(brief) > 100 else brief
                })

    return result


# ============================================================================
# 数据库操作
# ============================================================================

def create_mdx_tables(conn: sqlite3.Connection):
    """创建 MDX 词典表"""
    cursor = conn.cursor()

    # 柯林斯词典表
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS collins_words (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word TEXT NOT NULL,
            content TEXT NOT NULL,
            is_link INTEGER DEFAULT 0,
            link_target TEXT
        )
    """)
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_collins_word ON collins_words(word)")

    # 词根词缀词典表
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS etyma_words (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word TEXT NOT NULL,
            content TEXT NOT NULL,
            is_link INTEGER DEFAULT 0,
            link_target TEXT
        )
    """)
    cursor.execute("CREATE INDEX IF NOT EXISTS idx_etyma_word ON etyma_words(word)")

    conn.commit()


def import_collins(mdx_path: Path, conn: sqlite3.Connection) -> Tuple[int, int]:
    """导入柯林斯词典

    Returns:
        (imported_count, link_count)
    """
    print(f"正在读取柯林斯词典: {mdx_path}")
    mdx = MDX(str(mdx_path))

    cursor = conn.cursor()
    imported = 0
    links = 0

    items = list(mdx.items())
    total = len(items)
    print(f"总词条数: {total}")

    for i, (key, value) in enumerate(items):
        if i % 10000 == 0:
            print(f"  处理进度: {i}/{total}")

        word = key.decode('utf-8')
        html = value.decode('utf-8')

        # 检查是否为链接
        if html.startswith('@@@LINK='):
            link_target = html[8:].strip()
            cursor.execute(
                "INSERT INTO collins_words (word, content, is_link, link_target) VALUES (?, ?, 1, ?)",
                (word, "{}", link_target)
            )
            links += 1
        else:
            # 解析 HTML
            parsed = parse_collins_html(html)
            content = json.dumps(parsed, ensure_ascii=False)
            cursor.execute(
                "INSERT INTO collins_words (word, content, is_link, link_target) VALUES (?, ?, 0, NULL)",
                (word, content)
            )
            imported += 1

    conn.commit()
    return imported, links


def import_etyma(mdx_path: Path, conn: sqlite3.Connection) -> Tuple[int, int]:
    """导入词根词缀词典

    Returns:
        (imported_count, link_count)
    """
    print(f"正在读取词根词缀词典: {mdx_path}")
    mdx = MDX(str(mdx_path))

    cursor = conn.cursor()
    imported = 0
    links = 0

    items = list(mdx.items())
    total = len(items)
    print(f"总词条数: {total}")

    for i, (key, value) in enumerate(items):
        if i % 1000 == 0:
            print(f"  处理进度: {i}/{total}")

        word = key.decode('utf-8')
        html = value.decode('utf-8')

        # 跳过说明词条
        if word.startswith('00'):
            continue

        # 检查是否为链接
        if html.startswith('@@@LINK='):
            link_target = html[8:].strip()
            cursor.execute(
                "INSERT INTO etyma_words (word, content, is_link, link_target) VALUES (?, ?, 1, ?)",
                (word, "{}", link_target)
            )
            links += 1
        else:
            # 解析 HTML
            parsed = parse_etyma_html(html)
            content = json.dumps(parsed, ensure_ascii=False)
            cursor.execute(
                "INSERT INTO etyma_words (word, content, is_link, link_target) VALUES (?, ?, 0, NULL)",
                (word, content)
            )
            imported += 1

    conn.commit()
    return imported, links


def main():
    # 路径配置
    collins_path = Path(r"C:\Users\hwuu\Desktop\柯林斯英汉双解词典.mdx")
    etyma_path = Path(r"C:\Users\hwuu\Desktop\英语词根词缀词频.mdx")
    db_path = Path(r"C:\Users\hwuu\dev\hwuu\dictyy\src-tauri\resources\dict.db")

    # 检查文件
    if not collins_path.exists():
        print(f"错误: 找不到柯林斯词典文件 - {collins_path}")
        return
    if not etyma_path.exists():
        print(f"错误: 找不到词根词缀词典文件 - {etyma_path}")
        return
    if not db_path.exists():
        print(f"错误: 找不到数据库文件 - {db_path}")
        return

    # 连接数据库
    print(f"连接数据库: {db_path}")
    conn = sqlite3.connect(db_path)

    # 创建表
    create_mdx_tables(conn)

    # 清空现有数据（如果有）
    cursor = conn.cursor()
    cursor.execute("DELETE FROM collins_words")
    cursor.execute("DELETE FROM etyma_words")
    conn.commit()

    # 导入柯林斯
    print("\n=== 导入柯林斯词典 ===")
    collins_imported, collins_links = import_collins(collins_path, conn)
    print(f"柯林斯词典: 导入 {collins_imported} 词条, {collins_links} 链接")

    # 导入词根词缀
    print("\n=== 导入词根词缀词典 ===")
    etyma_imported, etyma_links = import_etyma(etyma_path, conn)
    print(f"词根词缀词典: 导入 {etyma_imported} 词条, {etyma_links} 链接")

    # 统计
    print("\n=== 统计信息 ===")
    cursor.execute("SELECT COUNT(*) FROM collins_words")
    print(f"collins_words 表: {cursor.fetchone()[0]} 条")
    cursor.execute("SELECT COUNT(*) FROM etyma_words")
    print(f"etyma_words 表: {cursor.fetchone()[0]} 条")

    conn.close()
    print(f"\n完成! 数据库: {db_path}")


if __name__ == "__main__":
    main()
