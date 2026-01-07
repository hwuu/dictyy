import { useEffect, useState, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emitTo, listen } from "@tauri-apps/api/event";
import { lookupAbstract, type WordAbstract } from "@/services/dictionary";

export function Bubble() {
  const [data, setData] = useState<WordAbstract | null>(null);
  const [loading, setLoading] = useState(true);
  const currentWordRef = useRef<string>("");
  const isFirstLoad = useRef(true);

  useEffect(() => {
    // 查询单词的函数
    async function lookup(word: string) {
      // 如果是相同的单词，不重新查询
      if (word.toLowerCase() === currentWordRef.current.toLowerCase()) {
        return;
      }

      currentWordRef.current = word;

      // 只有首次加载时才显示 loading，后续更新保持当前内容
      if (isFirstLoad.current) {
        setLoading(true);
      }

      try {
        // 从内存查询摘要（几乎瞬时）
        const abstract = await lookupAbstract(word);
        if (abstract) {
          setData(abstract);
        } else {
          // 未找到
          setData({
            word: word,
            phonetic: "",
            main_def: "",
            collins_def: "",
            etyma_def: "",
            gpt4_def: "",
          });
        }
      } catch (e) {
        console.error("Bubble lookup error:", e);
        setData({
          word: word,
          phonetic: "",
          main_def: "查询失败",
          collins_def: "",
          etyma_def: "",
          gpt4_def: "",
        });
      } finally {
        setLoading(false);
        isFirstLoad.current = false;
        // 数据准备好后显示窗口（不获取焦点，避免打断用户操作）
        getCurrentWindow().show();
      }
    }

    // 从 URL 获取初始单词
    const params = new URLSearchParams(window.location.search);
    const word = params.get("word");

    if (word) {
      lookup(word);
    }

    // 监听单词更新事件
    const unlisten = listen<string>("update-word", (event) => {
      lookup(event.payload);
    });

    // 气泡不获取焦点，关闭由 Rust 端的点击检测处理

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []); // 空依赖，只在挂载时运行一次

  // 点击详细按钮
  async function handleDetailClick() {
    if (data) {
      // 发送事件给主窗口
      await emitTo("main", "show-word-detail", { word: data.word });
      // 关闭气泡
      getCurrentWindow().close();
    }
  }

  // 获取显示的释义（优先级：main_def > collins_def > etyma_def > gpt4_def）
  function getDefinition(): string {
    if (!data) return "";
    if (data.main_def) return data.main_def;
    if (data.collins_def) return data.collins_def;
    if (data.etyma_def) return data.etyma_def;
    if (data.gpt4_def) return data.gpt4_def;
    return "未找到释义";
  }

  if (loading) {
    return (
      <div className="h-screen bg-background/95 backdrop-blur rounded-lg shadow-lg border p-3 flex items-center justify-center">
        <span className="text-sm text-muted-foreground">加载中...</span>
      </div>
    );
  }

  if (!data) {
    return null;
  }

  return (
    <div className="h-screen bg-background/95 backdrop-blur rounded-lg shadow-lg border p-3 flex flex-col">
      {/* 单词和音标 */}
      <div className="flex items-baseline gap-2">
        <span className="font-bold text-base">{data.word}</span>
        {data.phonetic && (
          <span className="text-sm text-muted-foreground">/{data.phonetic}/</span>
        )}
      </div>

      {/* 释义 */}
      <div className="flex-1 mt-1 text-sm text-foreground/90 line-clamp-3 overflow-hidden">
        {getDefinition()}
      </div>

      {/* 详细链接 */}
      <div className="flex justify-end mt-1">
        <button
          onClick={handleDetailClick}
          className="text-xs text-primary hover:underline cursor-pointer"
        >
          详细 →
        </button>
      </div>
    </div>
  );
}
