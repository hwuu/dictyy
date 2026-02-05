import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Loader2, X } from "lucide-react";
import { WordResult } from "@/components/WordResult";
import { CollinsResult } from "@/components/CollinsResult";
import { EtymaResult } from "@/components/EtymaResult";
import { Gpt4Result } from "@/components/Gpt4Result";
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
  lookupCollins,
  lookupEtyma,
  lookupGpt4,
  parseCollinsContent,
  parseEtymaContent,
  CollinsContent,
  EtymaContent,
} from "@/services/dictionary";

// Tab 类型
type TabType = "main" | "collins" | "etyma" | "gpt4" | "llm";

function App() {
  const [word, setWord] = useState("");
  const [searchedWord, setSearchedWord] = useState(""); // 当前查询的单词
  const [isSearching, setIsSearching] = useState(false);
  const [isLlmLoading, setIsLlmLoading] = useState(false);
  const [appVersion, setAppVersion] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const doSearchRef = useRef<(word: string) => void>(() => {});

  // 各数据源结果
  const [mainResult, setMainResult] = useState<ParsedWordContent | null>(null);
  const [collinsResult, setCollinsResult] = useState<CollinsContent | null>(null);
  const [etymaResult, setEtymaResult] = useState<EtymaContent | null>(null);
  const [gpt4Result, setGpt4Result] = useState<string | null>(null);
  const [llmResult, setLlmResult] = useState<ParsedWordContent | null>(null);

  // Tab 状态
  const [activeTab, setActiveTab] = useState<TabType>("main");

  // 搜索建议相关状态
  const [suggestions, setSuggestions] = useState<WordSuggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const debouncedWord = useDebounce(word, 200);

  // Listen for new-query event from Rust
  useEffect(() => {
    const unlisten = listen("new-query", () => {
      inputRef.current?.focus();
      inputRef.current?.select();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for show-word-detail event from bubble window
  useEffect(() => {
    console.log("[App] Setting up show-word-detail listener");
    const unlisten = listen<{ word: string }>("show-word-detail", async (event) => {
      console.log("[App] Received show-word-detail event:", event.payload);
      const wordToSearch = event.payload.word;
      console.log("[App] Word to search:", wordToSearch);

      // 显示主窗口
      try {
        console.log("[App] Getting current window...");
        const window = getCurrentWindow();
        console.log("[App] Window label:", window.label);

        console.log("[App] Calling window.show()...");
        await window.show();
        console.log("[App] window.show() completed");

        console.log("[App] Calling window.unminimize()...");
        await window.unminimize();
        console.log("[App] window.unminimize() completed");

        console.log("[App] Calling window.setFocus()...");
        await window.setFocus();
        console.log("[App] window.setFocus() completed");

        console.log("[App] Window shown and focused");
      } catch (error) {
        console.error("[App] Failed to show/focus window:", error);
        // 即使窗口操作失败，也继续查询
      }

      // 设置输入框并查询（先设置 searchedWord 防止触发搜索建议）
      console.log("[App] Setting state: searchedWord and word...");
      setSearchedWord(wordToSearch);
      setWord(wordToSearch);
      console.log("[App] State set, calling doSearch...");
      if (doSearchRef.current) {
        console.log("[App] Calling doSearch with:", wordToSearch);
        doSearchRef.current(wordToSearch);
      } else {
        console.error("[App] doSearchRef.current is undefined!");
      }
      console.log("[App] Event handler completed");
    });

    return () => {
      console.log("[App] Cleaning up show-word-detail listener");
      unlisten.then((fn) => fn());
    };
  }, []);

  // 获取版本号
  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch((err) => console.error("Failed to get app version:", err));
  }, []);

  // 搜索建议
  useEffect(() => {
    if (isSearching || isLlmLoading) {
      return;
    }

    // 如果已经搜索了这个词，不再显示建议
    if (searchedWord && searchedWord.toLowerCase() === debouncedWord.toLowerCase()) {
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
  }, [debouncedWord, isSearching, isLlmLoading, searchedWord]);

  // 处理键盘事件
  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (!showSuggestions) {
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
        if (selectedIndex >= 0 && suggestions.length > 0) {
          e.preventDefault();
          selectSuggestion(suggestions[selectedIndex].word);
        }
        break;
      case "Escape":
        e.preventDefault();
        setShowSuggestions(false);
        break;
    }
  }

  function selectSuggestion(selectedWord: string) {
    setWord(selectedWord);
    setShowSuggestions(false);
    doSearch(selectedWord);
  }

  async function doSearch(searchWord: string) {
    if (!searchWord.trim()) {
      clearResults();
      return;
    }

    const trimmedWord = searchWord.trim();
    setSearchedWord(trimmedWord);
    setIsSearching(true);
    setIsLlmLoading(false);
    clearResults();
    setShowSuggestions(false);
    setSuggestions([]);

    // 用于收集离线查询结果
    let hasMainResult = false;
    let hasCollinsResult = false;
    let hasEtymaResult = false;
    let hasGpt4Result = false;

    // 并行查询所有离线数据源
    const queries = [
      // 主词典
      lookupWord(trimmedWord).then((entry) => {
        if (entry) {
          setMainResult(parseWordContent(entry));
          hasMainResult = true;
        }
      }).catch(e => console.error("lookupWord error:", e)),

      // 柯林斯
      lookupCollins(trimmedWord).then((entry) => {
        if (entry) {
          const parsed = parseCollinsContent(entry);
          if (parsed) {
            setCollinsResult(parsed);
            hasCollinsResult = true;
          }
        }
      }).catch(e => console.error("lookupCollins error:", e)),

      // 词根词缀
      lookupEtyma(trimmedWord).then((entry) => {
        if (entry) {
          const parsed = parseEtymaContent(entry);
          if (parsed) {
            setEtymaResult(parsed);
            hasEtymaResult = true;
          }
        }
      }).catch(e => console.error("lookupEtyma error:", e)),

      // GPT4
      lookupGpt4(trimmedWord).then((content) => {
        if (content) {
          setGpt4Result(content);
          hasGpt4Result = true;
        }
      }).catch(e => console.error("lookupGpt4 error:", e)),
    ];

    try {
      await Promise.all(queries);

      // 如果离线词典都查不到，回退到 LLM
      const hasOfflineResult = hasMainResult || hasCollinsResult || hasEtymaResult || hasGpt4Result;
      if (!hasOfflineResult) {
        setIsSearching(false);
        setIsLlmLoading(true);
        try {
          const llmContent = await llmQuery(trimmedWord);
          setLlmResult(createLlmResult(trimmedWord, llmContent));
        } catch (llmError) {
          console.error("LLM query failed:", llmError);
        } finally {
          setIsLlmLoading(false);
        }
      }
    } catch (error) {
      console.error("Query failed:", error);
    } finally {
      setIsSearching(false);
      setIsLlmLoading(false);
    }
  }

  // 保持 doSearchRef 指向最新的 doSearch
  doSearchRef.current = doSearch;

  function clearResults() {
    setMainResult(null);
    setCollinsResult(null);
    setEtymaResult(null);
    setGpt4Result(null);
    setLlmResult(null);
    // 注意：不要在这里清空 searchedWord
  }

  // 判断是否有任何结果
  const hasAnyResult = mainResult || collinsResult || etymaResult || gpt4Result || llmResult;

  // 计算各 Tab 是否有内容
  const tabHasContent = {
    main: !!mainResult,
    collins: !!collinsResult,
    etyma: !!etymaResult,
    gpt4: !!gpt4Result,
    llm: !!llmResult,
  };

  // 获取第一个有内容的 Tab
  const getFirstAvailableTab = (): TabType => {
    if (mainResult) return "main";
    if (collinsResult) return "collins";
    if (etymaResult) return "etyma";
    if (gpt4Result) return "gpt4";
    if (llmResult) return "llm";
    return "main";
  };

  // 当结果变化时，自动切换到第一个有内容的 Tab
  useEffect(() => {
    if (hasAnyResult) {
      const firstTab = getFirstAvailableTab();
      if (!tabHasContent[activeTab]) {
        setActiveTab(firstTab);
      }
    }
  }, [mainResult, collinsResult, etymaResult, gpt4Result, llmResult]);

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
            doSearch(word);
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
                if (suggestions.length > 0 && !mainResult && !isSearching) {
                  setShowSuggestions(true);
                }
              }}
              onBlur={() => {
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

        <div className="mt-4 flex-1 overflow-hidden flex flex-col">
          {isSearching && (
            <p className="text-muted-foreground text-sm">查询中...</p>
          )}

          {isLlmLoading && (
            <p className="text-muted-foreground text-sm flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              词典未收录，正在请求 LLM...
            </p>
          )}

          {!isSearching && !isLlmLoading && searchedWord && !hasAnyResult && (
            <p className="text-muted-foreground text-sm">
              未找到: "{searchedWord}"
            </p>
          )}

          {!isSearching && !isLlmLoading && hasAnyResult && (
            <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as TabType)} className="flex-1 flex flex-col overflow-hidden">
              <TabsList className="w-full justify-start flex-shrink-0">
                {tabHasContent.main && (
                  <TabsTrigger value="main">主词典</TabsTrigger>
                )}
                {tabHasContent.collins && (
                  <TabsTrigger value="collins">柯林斯</TabsTrigger>
                )}
                {tabHasContent.etyma && (
                  <TabsTrigger value="etyma">词根词缀</TabsTrigger>
                )}
                {tabHasContent.gpt4 && (
                  <TabsTrigger value="gpt4">GPT4</TabsTrigger>
                )}
                {tabHasContent.llm && (
                  <TabsTrigger value="llm">LLM</TabsTrigger>
                )}
              </TabsList>

              <div className="flex-1 overflow-y-auto mt-4">
                <TabsContent value="main" className="mt-0">
                  {mainResult && <WordResult word={mainResult} />}
                </TabsContent>

                <TabsContent value="collins" className="mt-0">
                  {collinsResult && <CollinsResult content={collinsResult} />}
                </TabsContent>

                <TabsContent value="etyma" className="mt-0">
                  {etymaResult && <EtymaResult content={etymaResult} />}
                </TabsContent>

                <TabsContent value="gpt4" className="mt-0">
                  {gpt4Result && <Gpt4Result content={gpt4Result} />}
                </TabsContent>

                <TabsContent value="llm" className="mt-0">
                  {llmResult && <WordResult word={llmResult} />}
                </TabsContent>
              </div>
            </Tabs>
          )}

          {!searchedWord && !isSearching && (
            <div className="h-full flex flex-col items-center justify-center text-muted-foreground/50">
              <div className="text-4xl mb-2">📖</div>
              <p className="text-sm">查询单词或短语</p>
            </div>
          )}
        </div>
      </div>

      {/* Status Bar */}
      <div className="px-3 py-1 border-t bg-muted/30 text-xs text-muted-foreground flex justify-between">
        <div>
          <kbd className="px-1 py-0.5 bg-muted rounded">Ctrl+`</kbd> 隐藏
        </div>
        <div className="shrink-0">
          {appVersion && <span className="text-muted-foreground/60">v{appVersion}</span>}
        </div>
      </div>
    </div>
  );
}

export default App;
