// 词典服务 - 调用 Tauri 后端查询单词

import { invoke } from "@tauri-apps/api/core";

/** 单词条目 */
export interface WordEntry {
  word: string;
  phonetic_us: string | null;
  phonetic_uk: string | null;
  content: string;
  sources: string[];
  gpt4_content: string | null;
}

/** 搜索建议 */
export interface WordSuggestion {
  word: string;
  brief: string;
}

/** 解析后的单词内容 */
export interface ParsedWordContent {
  word: string;
  phoneticUs: string | null;
  phoneticUk: string | null;
  translations: Translation[];
  sentences: Sentence[];
  phrases: Phrase[];
  synonyms: Synonym[];
  relatedWords: RelatedWord[];
  rememberMethod: string | null;
  sources: string[];
  gpt4Content: string | null;
  llmContent: string | null; // LLM 回退内容
}

export interface Translation {
  pos: string;
  tranCn: string;
  tranOther: string | null;
}

export interface Sentence {
  en: string;
  cn: string;
}

export interface Phrase {
  phrase: string;
  meaning: string;
}

export interface Synonym {
  pos: string;
  words: string[];
}

export interface RelatedWord {
  pos: string;
  words: { word: string; meaning: string }[];
}

/** 查询单词 */
export async function lookupWord(word: string): Promise<WordEntry | null> {
  return invoke<WordEntry | null>("lookup_word", { word });
}

/** 搜索单词（模糊匹配） */
export async function searchWords(query: string): Promise<WordSuggestion[]> {
  return invoke<WordSuggestion[]>("search_words", { query });
}

/** LLM 查询 */
export async function llmQuery(word: string): Promise<string> {
  return invoke<string>("llm_query", { word });
}

/** 解析原始 JSON content */
export function parseWordContent(entry: WordEntry): ParsedWordContent {
  const raw = JSON.parse(entry.content);
  const wordContent = raw.content?.word?.content || {};

  // 解析翻译
  const translations: Translation[] = (wordContent.trans || []).map(
    (t: { pos?: string; tranCn?: string; tranOther?: string }) => ({
      pos: t.pos || "",
      tranCn: t.tranCn || "",
      tranOther: t.tranOther || null,
    })
  );

  // 解析例句
  const sentences: Sentence[] = (
    wordContent.sentence?.sentences || []
  ).map((s: { sContent?: string; sCn?: string }) => ({
    en: s.sContent || "",
    cn: s.sCn || "",
  }));

  // 解析短语
  const phrases: Phrase[] = (wordContent.phrase?.phrases || []).map(
    (p: { pContent?: string; pCn?: string }) => ({
      phrase: p.pContent || "",
      meaning: p.pCn || "",
    })
  );

  // 解析同义词
  const synonyms: Synonym[] = (wordContent.syno?.synos || []).map(
    (s: { pos?: string; tran?: string; hwds?: { w?: string }[] }) => ({
      pos: s.pos || "",
      words: (s.hwds || []).map((h: { w?: string }) => h.w || ""),
    })
  );

  // 解析相关词
  const relatedWords: RelatedWord[] = (wordContent.relWord?.rels || []).map(
    (r: { pos?: string; words?: { hwd?: string; tran?: string }[] }) => ({
      pos: r.pos || "",
      words: (r.words || []).map((w: { hwd?: string; tran?: string }) => ({
        word: w.hwd || "",
        meaning: w.tran || "",
      })),
    })
  );

  return {
    word: entry.word,
    phoneticUs: entry.phonetic_us,
    phoneticUk: entry.phonetic_uk,
    translations,
    sentences,
    phrases,
    synonyms,
    relatedWords,
    rememberMethod: wordContent.remMethod?.val || null,
    sources: entry.sources,
    gpt4Content: entry.gpt4_content,
    llmContent: null,
  };
}

/** 创建 LLM 回退结果 */
export function createLlmResult(word: string, llmContent: string): ParsedWordContent {
  return {
    word,
    phoneticUs: null,
    phoneticUk: null,
    translations: [],
    sentences: [],
    phrases: [],
    synonyms: [],
    relatedWords: [],
    rememberMethod: null,
    sources: ["LLM"],
    gpt4Content: null,
    llmContent,
  };
}
