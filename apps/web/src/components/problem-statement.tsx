"use client";

import { ChevronDown, ChevronRight } from "lucide-react";

interface Example {
  input: string;
  output: string;
}

interface ProblemStatementProps {
  title: string;
  difficulty: number;
  topic: string;
  description: string;
  inputSpec: string;
  outputSpec: string;
  examples: Example[];
  collapsed: boolean;
  onToggle: () => void;
}

function difficultyLabel(difficulty: number): string {
  if (difficulty <= 1200) return "Easy";
  if (difficulty <= 1800) return "Medium";
  return "Hard";
}

function difficultyColor(difficulty: number): string {
  if (difficulty <= 1200) return "text-emerald-400 bg-emerald-400/10 border-emerald-400/30";
  if (difficulty <= 1800) return "text-amber-400 bg-amber-400/10 border-amber-400/30";
  return "text-red-400 bg-red-400/10 border-red-400/30";
}

export function ProblemStatement({
  title,
  difficulty,
  topic,
  description,
  inputSpec,
  outputSpec,
  examples,
  collapsed,
  onToggle,
}: ProblemStatementProps) {
  return (
    <div className="flex flex-col bg-zinc-900 border border-zinc-700/50 rounded-md overflow-hidden">
      {/* Header */}
      <button
        type="button"
        onClick={onToggle}
        className="flex items-center gap-3 px-4 py-3 text-left hover:bg-zinc-800/60 transition-colors cursor-pointer select-none"
      >
        <span className="text-zinc-400 flex-shrink-0">
          {collapsed ? (
            <ChevronRight className="w-4 h-4" />
          ) : (
            <ChevronDown className="w-4 h-4" />
          )}
        </span>
        <span className="font-mono text-sm font-semibold text-zinc-100 flex-1 truncate">
          {title}
        </span>
        <span
          className={`flex-shrink-0 font-mono text-xs px-2 py-0.5 rounded border ${difficultyColor(difficulty)}`}
        >
          {difficultyLabel(difficulty)} · {difficulty}
        </span>
        <span className="flex-shrink-0 font-mono text-xs text-zinc-400 bg-zinc-800 px-2 py-0.5 rounded border border-zinc-700/50">
          {topic}
        </span>
      </button>

      {/* Body */}
      {!collapsed && (
        <div className="border-t border-zinc-700/50 px-4 py-4 flex flex-col gap-5 overflow-y-auto">
          {/* Description */}
          <section>
            <p className="font-mono text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed">
              {description}
            </p>
          </section>

          {/* Input spec */}
          <section>
            <h3 className="font-mono text-xs font-semibold uppercase tracking-widest text-zinc-500 mb-1.5">
              Input
            </h3>
            <p className="font-mono text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed">
              {inputSpec}
            </p>
          </section>

          {/* Output spec */}
          <section>
            <h3 className="font-mono text-xs font-semibold uppercase tracking-widest text-zinc-500 mb-1.5">
              Output
            </h3>
            <p className="font-mono text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed">
              {outputSpec}
            </p>
          </section>

          {/* Examples */}
          {examples.length > 0 && (
            <section>
              <h3 className="font-mono text-xs font-semibold uppercase tracking-widest text-zinc-500 mb-2.5">
                Examples
              </h3>
              <div className="flex flex-col gap-3">
                {examples.map((example, idx) => (
                  <div
                    key={idx}
                    className="rounded border border-zinc-700/50 overflow-hidden"
                  >
                    <div className="grid grid-cols-2 divide-x divide-zinc-700/50">
                      <div className="flex flex-col">
                        <span className="px-3 py-1.5 font-mono text-xs text-zinc-500 bg-zinc-800/60 border-b border-zinc-700/50">
                          Input #{idx + 1}
                        </span>
                        <pre className="px-3 py-2.5 font-mono text-xs text-zinc-200 bg-zinc-950/50 whitespace-pre overflow-x-auto">
                          {example.input}
                        </pre>
                      </div>
                      <div className="flex flex-col">
                        <span className="px-3 py-1.5 font-mono text-xs text-zinc-500 bg-zinc-800/60 border-b border-zinc-700/50">
                          Output #{idx + 1}
                        </span>
                        <pre className="px-3 py-2.5 font-mono text-xs text-zinc-200 bg-zinc-950/50 whitespace-pre overflow-x-auto">
                          {example.output}
                        </pre>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          )}
        </div>
      )}
    </div>
  );
}
