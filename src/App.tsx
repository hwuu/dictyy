import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Loader2, X } from "lucide-react";
import { WordResult } from "@/components/WordResult";
import { SearchSuggestions } from "@/components/SearchSuggestions";
import { useDebounce } from "@/hooks/useDebounce";
import {
  lookupWord,
  parseWordContent,
  ParsedWordContent,
  llmQuery,
  createLlmResult,
  searchWords,
  WordSuggestion,
  getLlmConfig,
  LlmConfigInfo,
} from "@/services/dictionary";

function App() {
  const [word, setWord] = useState("");
  const [isSearching, setIsSearching] = useState(false);
  const [isLlmLoading, setIsLlmLoading] = useState(false);
  const [result, setResult] = useState<ParsedWordContent | null>(null);
  const [notFound, setNotFound] = useState(false);
  const [llmConfig, setLlmConfig] = useState<LlmConfigInfo | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // 搜索建议相关状态
  const [suggestions, setSuggestions] = useState<WordSuggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(-1); // -1 表示未选中任何项
  const [showSuggestions, setShowSuggestions] = useState(false);
  const debouncedWord = useDebounce(word, 200);

  // Listen for new-query event from Rust
  useEffect(() => {
    const unlisten = listen("new-query", () => {
      // Focus input when window is shown via shortcut
      inputRef.current?.focus();
      inputRef.current?.select();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // 获取 LLM 配置
  useEffect(() => {
    getLlmConfig()
      .then(setLlmConfig)
      .catch((err) => console.error("Failed to get LLM config:", err));
  }, []);

  // 搜索建议
  useEffect(() => {
    // 正在查询中不显示建议
    if (isSearching || isLlmLoading) {
      return;
    }

    // 如果有结果，且输入内容和结果单词相同或是其前缀，不显示建议
    if (result && result.word.toLowerCase().startsWith(debouncedWord.toLowerCase())) {
      return;
    }

    if (debouncedWord.length < 2) {
      setSuggestions([]);
      setShowSuggestions(false);
      return;
    }

    searchWords(debouncedWord)
      .then((results) => {
        setSuggestions(results);
        setSelectedIndex(-1);
        setShowSuggestions(results.length > 0);
      })
      .catch((err) => {
        console.error("Search failed:", err);
        setSuggestions([]);
      });
  }, [debouncedWord, isSearching, isLlmLoading, result]);

  // 处理键盘事件
  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (!showSuggestions) {
      if (e.key === "Escape") {
        getCurrentWindow().hide();
      }
      return;
    }

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((prev) =>
          prev < suggestions.length - 1 ? prev + 1 : prev
        );
        break;
      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((prev) => (prev > -1 ? prev - 1 : prev));
        break;
      case "Tab":
        e.preventDefault();
        if (selectedIndex >= 0 && suggestions.length > 0) {
          setWord(suggestions[selectedIndex].word);
          setShowSuggestions(false);
        }
        break;
      case "Enter":
        // 只有选中了候选词才查询候选词，否则查询输入框内容
        if (selectedIndex >= 0 && suggestions.length > 0) {
          e.preventDefault();
          selectSuggestion(suggestions[selectedIndex].word);
        }
        // 未选中时不阻止默认行为，让表单提交处理
        break;
      case "Escape":
        e.preventDefault();
        setShowSuggestions(false);
        break;
    }
  }

  // 选择建议词并查询
  function selectSuggestion(selectedWord: string) {
    setWord(selectedWord);
    setShowSuggestions(false);
    doSearch(selectedWord);
  }

  async function doSearch(searchWord: string) {
    if (!searchWord.trim()) {
      // 清空结果，显示默认内容
      setResult(null);
      setNotFound(false);
      return;
    }
    setIsSearching(true);
    setIsLlmLoading(false);
    setNotFound(false);
    setResult(null);
    setShowSuggestions(false);
    setSuggestions([]); // 清空建议列表，防止重新显示

    try {
      const entry = await lookupWord(searchWord.trim());
      if (entry) {
        setResult(parseWordContent(entry));
      } else {
        // 离线词典找不到，尝试 LLM 回退
        setIsSearching(false);
        setIsLlmLoading(true);
        try {
          const llmContent = await llmQuery(searchWord.trim());
          setResult(createLlmResult(searchWord.trim(), llmContent));
        } catch (llmError) {
          console.error("LLM query failed:", llmError);
          setNotFound(true);
        }
      }
    } catch (error) {
      console.error("Lookup failed:", error);
      setNotFound(true);
    } finally {
      setIsSearching(false);
      setIsLlmLoading(false);
    }
  }

  async function handleSearch() {
    doSearch(word);
  }

  return (
    <div className="h-screen bg-background flex flex-col overflow-hidden">
      {/* Caption Bar - 可拖动 */}
      <div
        className="flex items-center justify-between px-3 py-1.5 bg-muted/50 border-b cursor-move select-none"
        onMouseDown={() => getCurrentWindow().startDragging()}
      >
        <span className="text-sm font-medium">Dictyy 词典</span>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 cursor-default hover:bg-foreground/10"
          onClick={() => getCurrentWindow().hide()}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* 主内容区 */}
      <div className="flex-1 p-4 overflow-hidden flex flex-col">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleSearch();
          }}
          className="flex gap-2 flex-shrink-0"
        >
          <div className="relative flex-1 min-w-0">
            <Input
              ref={inputRef}
              value={word}
              onChange={(e) => setWord(e.target.value)}
              onKeyDown={handleKeyDown}
              onFocus={() => {
                if (suggestions.length > 0 && !result && !isSearching) {
                  setShowSuggestions(true);
                }
              }}
              onBlur={() => {
                // 延迟关闭以允许点击建议
                setTimeout(() => setShowSuggestions(false), 150);
              }}
              placeholder="输入单词或短语..."
              className="w-full"
              autoFocus
            />
            <SearchSuggestions
              suggestions={suggestions}
              selectedIndex={selectedIndex}
              onSelect={selectSuggestion}
              visible={showSuggestions}
            />
          </div>
          <Button type="submit" disabled={isSearching} className="shrink-0">
            {isSearching ? "..." : "查询"}
          </Button>
        </form>

        <div className="mt-4 flex-1 overflow-y-auto">
          {isSearching && (
            <p className="text-muted-foreground text-sm">查询中...</p>
          )}
          {isLlmLoading && (
            <p className="text-muted-foreground text-sm flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              词典未收录，正在请求 LLM...
            </p>
          )}
          {notFound && (
            <p className="text-muted-foreground text-sm">
              未找到: "{word}"
            </p>
          )}
          {result && <WordResult word={result} />}
          {!result && !notFound && !isSearching && !isLlmLoading && (
            <div className="h-full flex flex-col items-center justify-center text-muted-foreground/50">
              <div className="text-4xl mb-2">📖</div>
              <p className="text-sm">查询单词或短语</p>
            </div>
          )}
        </div>
      </div>

      {/* Status Bar */}
      <div className="px-3 py-1 border-t bg-muted/30 text-xs text-muted-foreground flex justify-between">
        <div className="truncate">
          {llmConfig?.configured ? (
            <span>{llmConfig.api_base} | {llmConfig.model}</span>
          ) : (
            <span className="text-yellow-600">LLM 未配置</span>
          )}
        </div>
        <div className="flex gap-3 shrink-0">
          <span><kbd className="px-1 py-0.5 bg-muted rounded">Ctrl+`</kbd> 显示/隐藏</span>
          <span><kbd className="px-1 py-0.5 bg-muted rounded">Esc</kbd> 隐藏</span>
        </div>
      </div>
    </div>
  );
}

export default App;
