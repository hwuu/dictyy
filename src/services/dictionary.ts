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

/** MDX 词典条目 */
export interface MdxEntry {
  word: string;
  content: string;
  is_link: boolean;
  link_target: string | null;
}

/** 柯林斯词典解析后内容 */
export interface CollinsContent {
  word: string;
  phonetic_uk: string;
  phonetic_us: string;
  frequency: number;
  forms: string[];
  definitions: CollinsDefinition[];
}

export interface CollinsDefinition {
  num: string;
  pos: string;
  cn: string;
  en: string;
  examples: { en: string; cn: string }[];
  synonyms: string[];
}

/** 词根词缀词典解析后内容 */
export interface EtymaContent {
  word: string;
  pos: string;
  meaning: string;
  etymology: string;
  frequency: number;
  stars: number;
  root: string;
  related: { word: string; brief: string }[];
}

/** 单词摘要（气泡用） */
export interface WordAbstract {
  word: string;
  phonetic: string;
  main_def: string;
  collins_def: string;
  etyma_def: string;
  gpt4_def: string;
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

/** 查询柯林斯词典 */
export async function lookupCollins(word: string): Promise<MdxEntry | null> {
  return invoke<MdxEntry | null>("lookup_collins", { word });
}

/** 查询词根词缀词典 */
export async function lookupEtyma(word: string): Promise<MdxEntry | null> {
  return invoke<MdxEntry | null>("lookup_etyma", { word });
}

/** 查询 GPT4 词典 */
export async function lookupGpt4(word: string): Promise<string | null> {
  return invoke<string | null>("lookup_gpt4", { word });
}

/** 查询单词摘要（从内存） */
export async function lookupAbstract(word: string): Promise<WordAbstract | null> {
  return invoke<WordAbstract | null>("lookup_abstract", { word });
}

/** 解析柯林斯词典内容 */
export function parseCollinsContent(entry: MdxEntry): CollinsContent | null {
  if (entry.is_link || !entry.content) return null;
  try {
    return JSON.parse(entry.content) as CollinsContent;
  } catch {
    return null;
  }
}

/** 解析词根词缀词典内容 */
export function parseEtymaContent(entry: MdxEntry): EtymaContent | null {
  if (entry.is_link || !entry.content) return null;
  try {
    return JSON.parse(entry.content) as EtymaContent;
  } catch {
    return null;
  }
}

/** LLM 查询 */
export async function llmQuery(word: string): Promise<string> {
  return invoke<string>("llm_query", { word });
}

/** LLM 配置信息 */
export interface LlmConfigInfo {
  api_base: string;
  model: string;
  configured: boolean;
}

/** 获取 LLM 配置 */
export async function getLlmConfig(): Promise<LlmConfigInfo> {
  return invoke<LlmConfigInfo>("get_llm_config");
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
  // 尝试解析 JSON
  try {
    // 去掉可能的 markdown 代码块包裹
    let jsonStr = llmContent.trim();
    if (jsonStr.startsWith("```json")) {
      jsonStr = jsonStr.slice(7);
    } else if (jsonStr.startsWith("```")) {
      jsonStr = jsonStr.slice(3);
    }
    if (jsonStr.endsWith("```")) {
      jsonStr = jsonStr.slice(0, -3);
    }
    jsonStr = jsonStr.trim();

    const data = JSON.parse(jsonStr);

    return {
      word,
      phoneticUs: data.phonetic_us || null,
      phoneticUk: data.phonetic_uk || null,
      translations: (data.translations || []).map((t: { pos?: string; tranCn?: string }) => ({
        pos: t.pos || "",
        tranCn: t.tranCn || "",
        tranOther: null,
      })),
      sentences: (data.sentences || []).map((s: { en?: string; cn?: string }) => ({
        en: s.en || "",
        cn: s.cn || "",
      })),
      phrases: (data.phrases || []).map((p: { phrase?: string; meaning?: string }) => ({
        phrase: p.phrase || "",
        meaning: p.meaning || "",
      })),
      synonyms: [],
      relatedWords: [],
      rememberMethod: data.rememberMethod || null,
      sources: ["LLM"],
      gpt4Content: null,
      llmContent: null, // JSON 解析成功，不需要 fallback
    };
  } catch {
    // JSON 解析失败，降级为原始内容显示
    console.warn("Failed to parse LLM JSON response, falling back to raw content");
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
      llmContent, // 保留原始内容用于 fallback 显示
    };
  }
}
