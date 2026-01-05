import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
} from "@/services/dictionary";

function App() {
  const [word, setWord] = useState("");
  const [isSearching, setIsSearching] = useState(false);
  const [isLlmLoading, setIsLlmLoading] = useState(false);
  const [result, setResult] = useState<ParsedWordContent | null>(null);
  const [notFound, setNotFound] = useState(false);
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
    if (!searchWord.trim()) return;
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
    <div className="h-screen bg-background p-4 overflow-hidden">
      <Card className="w-full shadow-lg h-full flex flex-col">
        <CardHeader className="pb-2 flex-shrink-0 flex flex-row items-center justify-between">
          <CardTitle className="text-lg">Dictyy 词典</CardTitle>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => getCurrentWindow().hide()}
          >
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>
        <CardContent className="flex-1 overflow-hidden flex flex-col">
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
                placeholder="输入单词..."
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
              <div className="text-muted-foreground text-sm">
                <p>输入单词开始查询</p>
                <p className="mt-2 text-xs">
                  <kbd className="px-1 py-0.5 bg-muted rounded text-xs">Ctrl+`</kbd> 显示/隐藏
                  {" | "}
                  <kbd className="px-1 py-0.5 bg-muted rounded text-xs">Esc</kbd> 隐藏
                </p>
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export default App;
