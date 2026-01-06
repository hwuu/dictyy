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
  h1: ({ children }: { children?: React.ReactNode }) => (
    <h1 className="font-bold text-xl mt-4 mb-2">{children}</h1>
  ),
  h2: ({ children }: { children?: React.ReactNode }) => (
    <h2 className="font-bold text-lg mt-3 mb-2">{children}</h2>
  ),
  h3: ({ children }: { children?: React.ReactNode }) => (
    <h3 className="font-semibold text-base mt-3 mb-1">{children}</h3>
  ),
  h4: ({ children }: { children?: React.ReactNode }) => (
    <h4 className="font-semibold text-sm mt-2 mb-1">{children}</h4>
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
  em: ({ children }: { children?: React.ReactNode }) => (
    <em className="italic">{children}</em>
  ),
  code: ({ children }: { children?: React.ReactNode }) => (
    <code className="bg-muted px-1 py-0.5 rounded text-xs">{children}</code>
  ),
  hr: () => <hr className="my-3 border-border" />,
  blockquote: ({ children }: { children?: React.ReactNode }) => (
    <blockquote className="border-l-4 border-muted pl-3 my-2 text-muted-foreground">
      {children}
    </blockquote>
  ),
};

// 去掉 LLM 返回内容中的 markdown 代码块包裹
function stripMarkdownCodeBlock(content: string): string {
  // 去掉开头的 ```markdown 或 ```
  let cleaned = content.trim();
  if (cleaned.startsWith("```markdown")) {
    cleaned = cleaned.slice(11);
  } else if (cleaned.startsWith("```")) {
    cleaned = cleaned.slice(3);
  }
  // 去掉结尾的 ```
  if (cleaned.endsWith("```")) {
    cleaned = cleaned.slice(0, -3);
  }
  return cleaned.trim();
}

export function WordResult({ word }: WordResultProps) {
  // 如果是 LLM 回退结果，直接显示 LLM 内容
  if (word.llmContent) {
    const cleanContent = stripMarkdownCodeBlock(word.llmContent);
    return (
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold">{word.word}</h2>
          <Badge variant="outline" className="text-xs">
            LLM
          </Badge>
        </div>
        <div className="text-sm">
          <ReactMarkdown components={markdownComponents}>
            {cleanContent}
          </ReactMarkdown>
        </div>
      </div>
    );
  }

  // 计算默认展开的项
  const defaultOpenItems: string[] = [];
  if (word.sentences.length > 0) defaultOpenItems.push("sentences");
  if (word.phrases.length > 0) defaultOpenItems.push("phrases");
  if (word.rememberMethod) defaultOpenItems.push("remember");

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

        {/* 记忆技巧 */}
        {word.rememberMethod && (
          <AccordionItem value="remember">
            <AccordionTrigger className="text-sm py-2">
              记忆技巧
            </AccordionTrigger>
            <AccordionContent>
              <p className="text-sm">{word.rememberMethod}</p>
            </AccordionContent>
          </AccordionItem>
        )}

      </Accordion>
    </div>
  );
}
