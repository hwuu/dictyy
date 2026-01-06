import { EtymaContent } from "@/services/dictionary";
import { Badge } from "@/components/ui/badge";

interface EtymaResultProps {
  content: EtymaContent;
}

export function EtymaResult({ content }: EtymaResultProps) {
  // 词频星级显示
  const FrequencyStars = ({ count }: { count: number }) => (
    <span className="text-yellow-500">
      {"★".repeat(Math.min(count, 5))}
      {"☆".repeat(Math.max(0, 5 - count))}
    </span>
  );

  return (
    <div className="space-y-4">
      {/* 单词标题 */}
      <div>
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold">{content.word}</h2>
          {content.pos && (
            <Badge variant="secondary" className="text-xs">
              {content.pos}
            </Badge>
          )}
          {content.stars > 0 && <FrequencyStars count={content.stars} />}
        </div>

        {/* 释义 */}
        {content.meaning && (
          <p className="text-base font-medium mt-1">{content.meaning}</p>
        )}

        {/* 词频 */}
        {content.frequency > 0 && (
          <p className="text-sm text-muted-foreground">
            词频排名: #{content.frequency}
          </p>
        )}
      </div>

      {/* 词源解析 */}
      {content.etymology && (
        <div className="bg-primary/5 border border-primary/20 rounded-lg p-3">
          <h3 className="text-sm font-semibold mb-1 text-primary">词源解析</h3>
          <p className="text-sm">{content.etymology}</p>
        </div>
      )}

      {/* 词根说明 */}
      {content.root && (
        <div className="bg-muted/50 rounded-lg p-3">
          <h3 className="text-sm font-semibold mb-1">词根</h3>
          <p className="text-sm text-muted-foreground">{content.root}</p>
        </div>
      )}

      {/* 相关词汇 */}
      {content.related.length > 0 && (
        <div>
          <h3 className="text-sm font-semibold mb-2">相关词汇</h3>
          <div className="space-y-1">
            {content.related.slice(0, 10).map((rel, i) => (
              <div key={i} className="text-sm border-l-2 border-muted pl-2">
                <span className="font-medium">{rel.word}</span>
                {rel.brief && (
                  <span className="text-muted-foreground ml-2 text-xs">
                    {rel.brief.length > 60 ? rel.brief.slice(0, 60) + "..." : rel.brief}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
