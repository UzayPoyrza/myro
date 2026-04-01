"use client";

import { useCallback, useRef, useState } from "react";

interface CodeEditorProps {
  code: string;
  onChange: (code: string) => void;
  language?: string;
}

export function CodeEditor({ code, onChange, language = "python" }: CodeEditorProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [isFocused, setIsFocused] = useState(false);

  const lineCount = code === "" ? 1 : code.split("\n").length;

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Tab") {
        e.preventDefault();
        const textarea = e.currentTarget;
        const { selectionStart, selectionEnd } = textarea;
        const indent = "    "; // 4 spaces

        if (selectionStart === selectionEnd) {
          // No selection — just insert 4 spaces at cursor
          const newCode =
            code.slice(0, selectionStart) + indent + code.slice(selectionEnd);
          onChange(newCode);
          // Restore cursor position after React re-render
          requestAnimationFrame(() => {
            textarea.selectionStart = selectionStart + indent.length;
            textarea.selectionEnd = selectionStart + indent.length;
          });
        } else {
          // Selection spans multiple characters — indent each selected line
          const beforeSelection = code.slice(0, selectionStart);
          const selected = code.slice(selectionStart, selectionEnd);
          const afterSelection = code.slice(selectionEnd);

          if (e.shiftKey) {
            // Shift+Tab: dedent
            const dedented = selected.replace(/^ {1,4}/gm, "");
            const removed = selected.length - dedented.length;
            onChange(beforeSelection + dedented + afterSelection);
            requestAnimationFrame(() => {
              textarea.selectionStart = selectionStart;
              textarea.selectionEnd = selectionEnd - removed;
            });
          } else {
            const indented = selected.replace(/^/gm, indent);
            const added = indented.length - selected.length;
            onChange(beforeSelection + indented + afterSelection);
            requestAnimationFrame(() => {
              textarea.selectionStart = selectionStart;
              textarea.selectionEnd = selectionEnd + added;
            });
          }
        }
      }
    },
    [code, onChange]
  );

  return (
    <div
      className={`flex bg-zinc-950 rounded-md border overflow-hidden transition-colors ${
        isFocused
          ? "border-teal-500/40 ring-1 ring-teal-500/15"
          : "border-zinc-700/50"
      }`}
    >
      {/* Language badge in top-right corner */}
      <div className="absolute top-0 right-0 pointer-events-none" aria-hidden="true">
        {/* rendered via wrapper below */}
      </div>

      {/* Wrapper for badge positioning */}
      <div className="relative flex w-full">
        {/* Language badge */}
        <div className="absolute top-2 right-3 z-10 pointer-events-none">
          <span className="font-mono text-[10px] text-zinc-600 uppercase tracking-widest">
            {language}
          </span>
        </div>

        {/* Line numbers */}
        <div
          aria-hidden="true"
          className="flex-shrink-0 select-none bg-zinc-900/60 border-r border-zinc-800/60 px-3 py-3 text-right"
          style={{ minWidth: "3rem" }}
        >
          {Array.from({ length: lineCount }).map((_, i) => (
            <div
              key={i}
              className="font-mono text-xs text-zinc-700 leading-6 h-6"
              style={{ fontSize: "0.75rem", lineHeight: "1.5rem" }}
            >
              {i + 1}
            </div>
          ))}
        </div>

        {/* Textarea */}
        <textarea
          ref={textareaRef}
          value={code}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          spellCheck={false}
          autoCapitalize="none"
          autoCorrect="off"
          autoComplete="off"
          className="flex-1 font-mono text-xs text-zinc-200 bg-transparent resize-none outline-none px-4 py-3 leading-6 caret-teal-400 placeholder:text-zinc-700"
          style={{
            fontSize: "0.75rem",
            lineHeight: "1.5rem",
            minHeight: "300px",
            tabSize: 4,
          }}
          placeholder={`# Write your ${language} solution here\n`}
        />
      </div>
    </div>
  );
}
