import { WordSuggestion } from "@/services/dictionary";
import { cn } from "@/lib/utils";

interface SearchSuggestionsProps {
  suggestions: WordSuggestion[];
  selectedIndex: number; // -1 表示未选中任何项
  onSelect: (word: string) => void;
  visible: boolean;
}

export function SearchSuggestions({
  suggestions,
  selectedIndex,
  onSelect,
  visible,
}: SearchSuggestionsProps) {
  if (!visible || suggestions.length === 0) {
    return null;
  }

  return (
    <div className="absolute top-full left-0 right-0 mt-1 bg-background border rounded-md shadow-lg z-50 max-h-64 overflow-y-auto">
      {suggestions.map((suggestion, index) => (
        <div
          key={suggestion.word}
          className={cn(
            "px-3 py-2 cursor-pointer hover:bg-accent transition-colors",
            index === selectedIndex && "bg-accent"
          )}
          onMouseDown={(e) => {
            e.preventDefault();
            onSelect(suggestion.word);
          }}
        >
          <div className="font-medium text-sm">{suggestion.word}</div>
          {suggestion.brief && (
            <div className="text-xs text-muted-foreground truncate">
              {suggestion.brief}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
