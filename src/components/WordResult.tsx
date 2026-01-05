import { ParsedWordContent } from "@/services/dictionary";
import { Badge } from "@/components/ui/badge";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import ReactMarkdown from "react-markdown";

interface WordResultProps {
  word: ParsedWordContent;
}

// Markdown 组件配置
const markdownComponents = {
  h3: ({ children }: { children?: React.ReactNode }) => (
    <h3 className="font-semibold text-base mt-3 mb-1">{children}</h3>
  ),
  p: ({ children }: { children?: React.ReactNode }) => (
    <p className="my-1 leading-relaxed">{children}</p>
  ),
  ol: ({ children }: { children?: React.ReactNode }) => (
    <ol className="list-decimal list-inside my-2 space-y-1">{children}</ol>
  ),
  ul: ({ children }: { children?: React.ReactNode }) => (
    <ul className="list-disc list-inside my-2 space-y-1">{children}</ul>
  ),
  li: ({ children }: { children?: React.ReactNode }) => (
    <li className="ml-2">{children}</li>
  ),
  strong: ({ children }: { children?: React.ReactNode }) => (
    <strong className="font-semibold">{children}</strong>
  ),
  code: ({ children }: { children?: React.ReactNode }) => (
    <code className="bg-muted px-1 py-0.5 rounded text-xs">{children}</code>
  ),
};

export function WordResult({ word }: WordResultProps) {
  // 如果是 LLM 回退结果，直接显示 LLM 内容
  if (word.llmContent) {
    return (
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold">{word.word}</h2>
          <Badge variant="outline" className="text-xs">
            LLM
          </Badge>
        </div>
        <div className="text-sm markdown-content">
          <ReactMarkdown components={markdownComponents}>
            {word.llmContent}
          </ReactMarkdown>
        </div>
      </div>
    );
  }

  // 计算默认展开的项
  const defaultOpenItems: string[] = [];
  if (word.sentences.length > 0) defaultOpenItems.push("sentences");
  if (word.phrases.length > 0) defaultOpenItems.push("phrases");
  if (word.gpt4Content) defaultOpenItems.push("gpt4");

  return (
    <div className="space-y-3">
      {/* 单词标题和音标 */}
      <div>
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold">{word.word}</h2>
          <div className="flex gap-1">
            {word.sources.map((source) => (
              <Badge key={source} variant="secondary" className="text-xs">
                {source}
              </Badge>
            ))}
          </div>
        </div>
        <div className="text-sm text-muted-foreground">
          {word.phoneticUs && <span>美: /{word.phoneticUs}/</span>}
          {word.phoneticUs && word.phoneticUk && <span className="mx-2">|</span>}
          {word.phoneticUk && <span>英: /{word.phoneticUk}/</span>}
        </div>
      </div>

      {/* 释义 */}
      <div>
        {word.translations.map((t, i) => (
          <div key={i} className="text-sm">
            <span className="text-muted-foreground">{t.pos}</span>{" "}
            <span>{t.tranCn}</span>
          </div>
        ))}
      </div>

      {/* 可展开内容 - 默认全部展开 */}
      <Accordion type="multiple" defaultValue={defaultOpenItems} className="w-full">
        {/* 例句 */}
        {word.sentences.length > 0 && (
          <AccordionItem value="sentences">
            <AccordionTrigger className="text-sm py-2">
              例句 ({word.sentences.length})
            </AccordionTrigger>
            <AccordionContent>
              <div className="space-y-2">
                {word.sentences.slice(0, 3).map((s, i) => (
                  <div key={i} className="text-sm">
                    <p>{s.en}</p>
                    <p className="text-muted-foreground">{s.cn}</p>
                  </div>
                ))}
              </div>
            </AccordionContent>
          </AccordionItem>
        )}

        {/* 短语 */}
        {word.phrases.length > 0 && (
          <AccordionItem value="phrases">
            <AccordionTrigger className="text-sm py-2">
              短语 ({word.phrases.length})
            </AccordionTrigger>
            <AccordionContent>
              <div className="space-y-1">
                {word.phrases.slice(0, 5).map((p, i) => (
                  <div key={i} className="text-sm">
                    <span className="font-medium">{p.phrase}</span>
                    <span className="text-muted-foreground ml-2">{p.meaning}</span>
                  </div>
                ))}
              </div>
            </AccordionContent>
          </AccordionItem>
        )}

        {/* GPT4 解释 */}
        {word.gpt4Content && (
          <AccordionItem value="gpt4">
            <AccordionTrigger className="text-sm py-2">
              GPT4 解析
            </AccordionTrigger>
            <AccordionContent>
              <div className="text-sm markdown-content">
                <ReactMarkdown components={markdownComponents}>
                  {word.gpt4Content}
                </ReactMarkdown>
              </div>
            </AccordionContent>
          </AccordionItem>
        )}
      </Accordion>
    </div>
  );
}
