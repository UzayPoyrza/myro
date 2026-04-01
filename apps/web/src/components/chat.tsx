"use client";

import { useEffect, useRef, useState } from "react";
import { Send } from "lucide-react";

interface Message {
  id: string;
  role: "user" | "coach";
  content: string;
  createdAt: string;
}

interface ChatProps {
  messages: Message[];
  onSend: (message: string) => void;
  isLoading: boolean;
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return "";
  }
}

function TypingIndicator() {
  return (
    <div className="flex items-start gap-2.5 max-w-[75%]">
      <div className="w-6 h-6 rounded-full bg-teal-500/20 border border-teal-500/40 flex-shrink-0 flex items-center justify-center">
        <span className="font-mono text-[9px] text-teal-400 font-bold leading-none">M</span>
      </div>
      <div className="bg-zinc-700/60 border border-zinc-600/50 rounded-lg rounded-tl-none px-3.5 py-2.5">
        <div className="flex items-center gap-1 h-4">
          <span className="w-1.5 h-1.5 rounded-full bg-zinc-400 animate-bounce [animation-delay:0ms]" />
          <span className="w-1.5 h-1.5 rounded-full bg-zinc-400 animate-bounce [animation-delay:150ms]" />
          <span className="w-1.5 h-1.5 rounded-full bg-zinc-400 animate-bounce [animation-delay:300ms]" />
        </div>
      </div>
    </div>
  );
}

export function Chat({ messages, onSend, isLoading }: ChatProps) {
  const [draft, setDraft] = useState("");
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll to bottom whenever messages change or loading state changes
  useEffect(() => {
    const el = listRef.current;
    if (el) {
      el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
    }
  }, [messages, isLoading]);

  function handleSend() {
    const trimmed = draft.trim();
    if (!trimmed || isLoading) return;
    onSend(trimmed);
    setDraft("");
    inputRef.current?.focus();
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  return (
    <div className="flex flex-col h-full bg-zinc-900 border border-zinc-700/50 rounded-md overflow-hidden">
      {/* Header */}
      <div className="px-4 py-2.5 border-b border-zinc-700/50 bg-zinc-800/40 flex-shrink-0">
        <span className="font-mono text-xs font-semibold uppercase tracking-widest text-zinc-400">
          Coach
        </span>
      </div>

      {/* Messages */}
      <div
        ref={listRef}
        className="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-4 min-h-0"
      >
        {messages.length === 0 && !isLoading && (
          <div className="flex-1 flex items-center justify-center">
            <p className="font-mono text-xs text-zinc-600">
              Ask the coach for a hint or guidance.
            </p>
          </div>
        )}

        {messages.map((msg) => {
          const isUser = msg.role === "user";
          return (
            <div
              key={msg.id}
              className={`flex items-end gap-2 ${isUser ? "flex-row-reverse" : "flex-row"}`}
            >
              {/* Avatar */}
              {!isUser && (
                <div className="w-6 h-6 rounded-full bg-teal-500/20 border border-teal-500/40 flex-shrink-0 flex items-center justify-center mb-0.5">
                  <span className="font-mono text-[9px] text-teal-400 font-bold leading-none">
                    M
                  </span>
                </div>
              )}

              {/* Bubble */}
              <div className={`flex flex-col gap-1 max-w-[75%] ${isUser ? "items-end" : "items-start"}`}>
                <div
                  className={`px-3.5 py-2.5 rounded-lg font-mono text-sm leading-relaxed whitespace-pre-wrap break-words ${
                    isUser
                      ? "bg-teal-500/15 border border-teal-500/30 text-teal-100 rounded-br-none"
                      : "bg-zinc-700/60 border border-zinc-600/50 text-zinc-200 rounded-tl-none"
                  }`}
                >
                  {msg.content}
                </div>
                <span className="font-mono text-[10px] text-zinc-600 px-0.5">
                  {formatTime(msg.createdAt)}
                </span>
              </div>
            </div>
          );
        })}

        {isLoading && <TypingIndicator />}
      </div>

      {/* Input area */}
      <div className="flex-shrink-0 border-t border-zinc-700/50 px-3 py-3 bg-zinc-800/30">
        <div className="flex items-end gap-2">
          <textarea
            ref={inputRef}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={isLoading}
            rows={1}
            placeholder="Ask for a hint… (Enter to send, Shift+Enter for newline)"
            className="flex-1 resize-none font-mono text-sm bg-zinc-950/60 border border-zinc-700/50 rounded text-zinc-200 placeholder:text-zinc-600 px-3 py-2 outline-none focus:border-teal-500/50 focus:ring-1 focus:ring-teal-500/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed min-h-[36px] max-h-[120px] overflow-y-auto"
            style={{ height: "auto" }}
            onInput={(e) => {
              const t = e.currentTarget;
              t.style.height = "auto";
              t.style.height = `${Math.min(t.scrollHeight, 120)}px`;
            }}
          />
          <button
            type="button"
            onClick={handleSend}
            disabled={isLoading || !draft.trim()}
            className="flex-shrink-0 w-9 h-9 flex items-center justify-center rounded bg-teal-600 hover:bg-teal-500 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            aria-label="Send message"
          >
            <Send className="w-4 h-4 text-white" />
          </button>
        </div>
        <p className="font-mono text-[10px] text-zinc-600 mt-1.5 px-0.5">
          Shift+Enter for newline
        </p>
      </div>
    </div>
  );
}
