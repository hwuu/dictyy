import { useEffect, useState, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { lookupAbstract, type WordAbstract } from "@/services/dictionary";

export function Bubble() {
  const [data, setData] = useState<WordAbstract | null>(null);
  const [loading, setLoading] = useState(true);
  const currentWordRef = useRef<string>("");
  const isFirstLoad = useRef(true);

  useEffect(() => {
    console.log("[Bubble] Component mounted");

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "F12") {
        console.log("[Bubble] F12 pressed");
        getCurrentWindow().emit("toggle-devtools");
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    async function lookup(word: string) {
      if (word.toLowerCase() === currentWordRef.current.toLowerCase()) {
        return;
      }

      currentWordRef.current = word;

      if (isFirstLoad.current) {
        setLoading(true);
      }

      try {
        const abstract = await lookupAbstract(word);
        if (abstract) {
          setData(abstract);
        } else {
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
        getCurrentWindow().show();
      }
    }

    const params = new URLSearchParams(window.location.search);
    const word = params.get("word");

    if (word) {
      lookup(word);
    }

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  async function handleDetailClick() {
    console.log("[Bubble] handleDetailClick called, data:", data);
    if (data) {
      console.log("[Bubble] Calling Rust command show_main_window for:", data.word);
      try {
        // 调用 Rust 命令显示主窗口
        await invoke("show_main_window", { word: data.word });
        console.log("[Bubble] Rust command succeeded, now closing bubble...");
        await getCurrentWindow().close();
        console.log("[Bubble] Bubble closed");
      } catch (error) {
        console.error("[Bubble] Error:", error);
      }
    } else {
      console.error("[Bubble] No data available");
    }
  }

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
      <div className="flex items-baseline gap-2">
        <span className="font-bold text-base">{data.word}</span>
        {data.phonetic && (
          <span className="text-sm text-muted-foreground">/{data.phonetic}/</span>
        )}
      </div>

      <div className="flex-1 mt-1 text-sm text-foreground/90 line-clamp-3 overflow-hidden">
        {getDefinition()}
      </div>

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
