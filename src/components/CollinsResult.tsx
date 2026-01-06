import { CollinsContent, CollinsDefinition } from "@/services/dictionary";
import { Badge } from "@/components/ui/badge";

interface CollinsResultProps {
  content: CollinsContent;
}

// 词性优先级排序（数字越小优先级越高）
const POS_PRIORITY: Record<string, number> = {
  "VERB": 1,
  "N-COUNT": 2,
  "N-UNCOUNT": 3,
  "N-VAR": 4,
  "N-SING": 5,
  "N-PLURAL": 6,
  "N-PROPER": 7,
  "ADJ": 8,
  "ADV": 9,
  "PREP": 10,
  "CONJ": 11,
  "PRON": 12,
  "DET": 13,
  "QUANT": 14,
  "NUM": 15,
  "MODAL": 16,
  "AUX": 17,
  "EXCLAIM": 18,
  "PHRASE": 100,  // PHRASE 排在普通词性后面
  "See also:": 200,  // See Also 排在最后
};

function getPOSPriority(pos: string): number {
  // 尝试精确匹配
  if (POS_PRIORITY[pos] !== undefined) {
    return POS_PRIORITY[pos];
  }
  // 尝试前缀匹配（如 "N-COUNT" 匹配 "N-"）
  for (const [key, value] of Object.entries(POS_PRIORITY)) {
    if (pos.startsWith(key)) {
      return value;
    }
  }
  // 检查是否是 PHRASE 相关
  if (pos.includes("PHRASE")) {
    return POS_PRIORITY["PHRASE"];
  }
  // 检查是否是 See also
  if (pos.toLowerCase().includes("see also")) {
    return POS_PRIORITY["See also:"];
  }
  // 默认优先级
  return 50;
}

export function CollinsResult({ content }: CollinsResultProps) {
  // 词频星级显示
  const FrequencyStars = ({ count }: { count: number }) => (
    <span className="text-yellow-500">
      {"★".repeat(count)}
      {"☆".repeat(5 - count)}
    </span>
  );

  // 过滤词形变化中的无效字符（如图标字体字符）
  const validForms = content.forms.filter(f => f && f.trim() && /^[a-zA-Z\-' ]+$/.test(f));

  // 按词性分组并排序
  const sortedDefinitions = [...content.definitions].sort((a, b) => {
    const priorityA = getPOSPriority(a.pos);
    const priorityB = getPOSPriority(b.pos);
    if (priorityA !== priorityB) {
      return priorityA - priorityB;
    }
    // 同词性按原始顺序（num）排序
    return parseInt(a.num || "0") - parseInt(b.num || "0");
  });

  // 分离普通释义、PHRASE 和 See Also
  const regularDefs: CollinsDefinition[] = [];
  const phraseDefs: CollinsDefinition[] = [];
  const seeAlsoDefs: CollinsDefinition[] = [];

  for (const def of sortedDefinitions) {
    const priority = getPOSPriority(def.pos);
    if (priority >= 200) {
      seeAlsoDefs.push(def);
    } else if (priority >= 100) {
      phraseDefs.push(def);
    } else {
      regularDefs.push(def);
    }
  }

  // 渲染单个释义
  const renderDefinition = (def: CollinsDefinition, i: number) => (
    <div key={i} className="border-l-2 border-primary/30 pl-3">
      {/* 序号、词性、中文释义 - 同一行 */}
      <div className="flex items-center gap-2 flex-wrap">
        {def.num && (
          <Badge variant="outline" className="text-xs">
            {def.num}
          </Badge>
        )}
        {def.pos && (
          <Badge variant="secondary" className="text-xs">
            {def.pos}
          </Badge>
        )}
        {def.cn && (
          <span className="text-sm font-medium text-primary">{def.cn}</span>
        )}
      </div>

      {/* 英文释义 */}
      {def.en && (
        <p className="text-sm text-muted-foreground mt-1">{def.en}</p>
      )}

      {/* 例句 */}
      {def.examples.length > 0 && (
        <div className="mt-2 space-y-1">
          {def.examples.slice(0, 2).map((ex, j) => (
            <div key={j} className="text-sm bg-muted/30 rounded p-2">
              <p>{ex.en}</p>
              <p className="text-muted-foreground">{ex.cn}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  return (
    <div className="space-y-4">
      {/* 单词标题和音标 */}
      <div>
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold">{content.word}</h2>
          <FrequencyStars count={content.frequency} />
        </div>
        <div className="text-sm text-muted-foreground">
          {content.phonetic_uk && <span>英: /{content.phonetic_uk}/</span>}
          {content.phonetic_uk && content.phonetic_us && <span className="mx-2">|</span>}
          {content.phonetic_us && <span>美: /{content.phonetic_us}/</span>}
        </div>
        {validForms.length > 0 && (
          <div className="text-sm text-muted-foreground mt-1">
            词形变化: {validForms.join(", ")}
          </div>
        )}
      </div>

      {/* 普通释义（按词性排序） */}
      {regularDefs.length > 0 && (
        <div className="space-y-3">
          {regularDefs.map(renderDefinition)}
        </div>
      )}

      {/* PHRASE 短语 */}
      {phraseDefs.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-semibold text-muted-foreground border-b pb-1">短语</h3>
          {phraseDefs.map(renderDefinition)}
        </div>
      )}

      {/* See Also */}
      {seeAlsoDefs.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-semibold text-muted-foreground border-b pb-1">参见</h3>
          {seeAlsoDefs.map((def, i) => (
            <div key={i} className="text-sm text-muted-foreground">
              {def.en}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
