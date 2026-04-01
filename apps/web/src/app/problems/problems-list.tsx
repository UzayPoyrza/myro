"use client";

import { useState, useMemo, useEffect, useRef } from "react";
import Link from "next/link";

type ProblemData = {
  id: string;
  title: string;
  difficulty: number;
  topic: string;
  description: string;
  routeCount: number;
  totalObs: number;
};

function getDifficultyColor(difficulty: number): string {
  if (difficulty <= 1200) return "#00e5a0";
  if (difficulty <= 1500) return "#f59e0b";
  return "#ef4444";
}

function getDifficultyLabel(difficulty: number): string {
  if (difficulty <= 1200) return "easy";
  if (difficulty <= 1500) return "med";
  return "hard";
}

function getDifficultyBucket(difficulty: number): "easy" | "med" | "hard" {
  if (difficulty <= 1200) return "easy";
  if (difficulty <= 1500) return "med";
  return "hard";
}

export function ProblemsList({ problems }: { problems: ProblemData[] }) {
  const [search, setSearch] = useState("");
  const [activeTopics, setActiveTopics] = useState<Set<string>>(new Set());
  const [activeDifficulty, setActiveDifficulty] = useState<
    "all" | "easy" | "med" | "hard"
  >("all");
  const searchRef = useRef<HTMLInputElement>(null);

  // Ctrl+K / Cmd+K to focus search
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        searchRef.current?.focus();
      }
      // Escape to blur
      if (e.key === "Escape") {
        searchRef.current?.blur();
        setSearch("");
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  // Extract all unique topics
  const allTopics = useMemo(() => {
    const topicSet = new Set<string>();
    problems.forEach((p) => {
      p.topic.split(", ").forEach((t) => topicSet.add(t.trim()));
    });
    return Array.from(topicSet).sort();
  }, [problems]);

  // Filter problems
  const filtered = useMemo(() => {
    return problems.filter((p) => {
      // Search filter
      if (search) {
        const q = search.toLowerCase();
        const matchesTitle = p.title.toLowerCase().includes(q);
        const matchesTopic = p.topic.toLowerCase().includes(q);
        const matchesDifficulty = String(p.difficulty).includes(q);
        if (!matchesTitle && !matchesTopic && !matchesDifficulty) return false;
      }
      // Topic filter
      if (activeTopics.size > 0) {
        const problemTopics = p.topic.split(", ").map((t) => t.trim());
        if (!problemTopics.some((t) => activeTopics.has(t))) return false;
      }
      // Difficulty filter
      if (activeDifficulty !== "all") {
        if (getDifficultyBucket(p.difficulty) !== activeDifficulty) return false;
      }
      return true;
    });
  }, [problems, search, activeTopics, activeDifficulty]);

  // Difficulty counts
  const counts = useMemo(() => {
    const c = { all: problems.length, easy: 0, med: 0, hard: 0 };
    problems.forEach((p) => {
      c[getDifficultyBucket(p.difficulty)]++;
    });
    return c;
  }, [problems]);

  function toggleTopic(topic: string) {
    setActiveTopics((prev) => {
      const next = new Set(prev);
      if (next.has(topic)) next.delete(topic);
      else next.add(topic);
      return next;
    });
  }

  return (
    <>
      {/* Search + Difficulty filters row */}
      <div className="fade-in-delay-1 flex flex-col sm:flex-row gap-4 mb-6">
        {/* Search bar — terminal style */}
        <div className="relative flex-1 group">
          <div className="absolute left-4 top-1/2 -translate-y-1/2 text-[#00e5a0] pointer-events-none">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
          </div>
          <input
            ref={searchRef}
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search problems..."
            className="w-full bg-[#0a0f1a] border border-[#1e293b] rounded-sm pl-11 pr-20 py-3 font-mono text-sm text-[#e2e8f0] placeholder-[#334155] focus:outline-none focus:border-[#00e5a0] focus:shadow-[0_0_20px_#00e5a015] transition-[border-color,box-shadow] duration-300"
          />
          <div className="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-1 pointer-events-none">
            <kbd className="font-mono text-[10px] text-[#334155] bg-[#111827] border border-[#1e293b] px-1.5 py-0.5 rounded-sm">
              {typeof navigator !== "undefined" &&
              /Mac|iPhone/.test(navigator.userAgent)
                ? "\u2318"
                : "Ctrl"}
            </kbd>
            <kbd className="font-mono text-[10px] text-[#334155] bg-[#111827] border border-[#1e293b] px-1.5 py-0.5 rounded-sm">
              K
            </kbd>
          </div>
        </div>

        {/* Difficulty tabs */}
        <div className="flex items-center bg-[#0a0f1a] border border-[#1e293b] rounded-sm overflow-hidden flex-shrink-0">
          {(
            [
              { key: "all", label: "All", color: "#e2e8f0" },
              { key: "easy", label: "Easy", color: "#00e5a0" },
              { key: "med", label: "Med", color: "#f59e0b" },
              { key: "hard", label: "Hard", color: "#ef4444" },
            ] as const
          ).map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveDifficulty(tab.key)}
              className={`font-mono text-[11px] px-4 py-3 transition-colors duration-200 cursor-pointer relative ${
                activeDifficulty === tab.key
                  ? "bg-[#111827] text-white"
                  : "text-[#64748b] hover:text-[#94a3b8] hover:bg-[#0d1220]"
              }`}
            >
              <span className="flex items-center gap-1.5">
                {tab.key !== "all" && (
                  <span
                    className="inline-block w-1.5 h-1.5 rounded-full"
                    style={{ backgroundColor: tab.color }}
                  />
                )}
                {tab.label}
                <span
                  className={`text-[10px] ${
                    activeDifficulty === tab.key
                      ? "text-[#94a3b8]"
                      : "text-[#334155]"
                  }`}
                >
                  {counts[tab.key]}
                </span>
              </span>
              {activeDifficulty === tab.key && (
                <div
                  className="absolute bottom-0 left-0 right-0 h-[2px]"
                  style={{ backgroundColor: tab.color }}
                />
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Topic pills */}
      <div className="fade-in-delay-2 flex flex-wrap gap-2 mb-8">
        {allTopics.map((tag) => {
          const isActive = activeTopics.has(tag);
          return (
            <button
              key={tag}
              onClick={() => toggleTopic(tag)}
              className={`font-mono text-[11px] px-3 py-1.5 bg-[#0a0f1a] border border-[#1e293b] rounded-sm transition-colors duration-200 cursor-pointer ${
                isActive
                  ? "border-[#00e5a050] text-[#00e5a0]"
                  : "text-[#64748b] hover:border-[#00e5a030] hover:text-[#94a3b8]"
              }`}
            >
              {tag}
            </button>
          );
        })}
        <button
          onClick={() => setActiveTopics(new Set())}
          className={`font-mono text-[11px] px-3 py-1.5 transition-colors cursor-pointer ${
            activeTopics.size > 0
              ? "text-[#ef4444] hover:text-[#f87171]"
              : "text-transparent pointer-events-none"
          }`}
        >
          clear all
        </button>
      </div>

      {/* Results count */}
      <div className="fade-in-delay-2 flex items-center gap-3 mb-5">
        <span className="font-mono text-[11px] text-[#64748b] tabular-nums">
          {filtered.length} of {problems.length} problem{problems.length !== 1 ? "s" : ""}
        </span>
        <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
      </div>

      {/* Problem list */}
      <div className="space-y-2">
        {filtered.map((problem, i) => {
          const diffColor = getDifficultyColor(problem.difficulty);
          const diffLabel = getDifficultyLabel(problem.difficulty);

          return (
            <Link
              key={problem.id}
              href={`/problems/${problem.id}/start`}
              className="group block bg-[#0a0f1a] border border-[#1e293b] rounded-sm transition-[background-color,border-color,box-shadow] duration-300 hover:bg-[#0d1220] hover:border-[#1e293b80] hover:shadow-[0_0_30px_#00e5a008]"
              style={{
                borderLeftWidth: "3px",
                borderLeftColor: diffColor,
                animationDelay: `${Math.min(i * 0.05, 0.4)}s`,
              }}
            >
              <div className="p-5 flex items-center gap-5">
                {/* Difficulty badge — left */}
                <div className="flex-shrink-0 hidden sm:flex flex-col items-center w-14">
                  <span
                    className="font-display text-lg font-bold leading-none"
                    style={{ color: diffColor }}
                  >
                    {problem.difficulty}
                  </span>
                  <span
                    className="font-mono text-[9px] uppercase tracking-widest mt-1"
                    style={{ color: `${diffColor}99` }}
                  >
                    {diffLabel}
                  </span>
                </div>

                {/* Separator */}
                <div className="hidden sm:block w-[1px] h-10 bg-[#1e293b]" />

                {/* Content */}
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-3 mb-1.5">
                    <h2 className="font-mono text-sm font-semibold text-[#e2e8f0] group-hover:text-[#00e5a0] transition-colors truncate">
                      {problem.title}
                    </h2>
                    {/* Mobile difficulty badge */}
                    <span
                      className="sm:hidden font-mono text-[10px] font-bold px-1.5 py-0.5 rounded-sm border flex-shrink-0"
                      style={{
                        color: diffColor,
                        borderColor: `${diffColor}40`,
                        backgroundColor: `${diffColor}10`,
                      }}
                    >
                      {problem.difficulty}
                    </span>
                  </div>
                  <p className="font-mono text-xs text-[#64748b] leading-relaxed line-clamp-1 mb-2.5">
                    {problem.description.slice(0, 120)}
                    {problem.description.length > 120 ? "..." : ""}
                  </p>
                  <div className="flex items-center gap-3">
                    <div className="flex flex-wrap gap-1.5">
                      {problem.topic.split(", ").map((tag) => (
                        <span
                          key={tag}
                          className="font-mono text-[10px] px-1.5 py-0.5 bg-[#111827] text-[#64748b] border border-[#1e293b] rounded-sm"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                    <span className="font-mono text-[10px] text-[#334155] flex-shrink-0 ml-auto">
                      {problem.routeCount} route
                      {problem.routeCount !== 1 ? "s" : ""} &middot;{" "}
                      {problem.totalObs} insight
                      {problem.totalObs !== 1 ? "s" : ""}
                    </span>
                  </div>
                </div>

                {/* Arrow — right */}
                <div className="flex-shrink-0 text-[#1e293b] group-hover:text-[#00e5a0] transition-[color,transform] duration-300 group-hover:translate-x-1">
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                </div>
              </div>
            </Link>
          );
        })}

        {/* Empty states */}
        {filtered.length === 0 && problems.length > 0 && (
          <div className="text-center py-16">
            <div className="text-[#1e293b] mb-4">
              <svg
                width="40"
                height="40"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="mx-auto"
              >
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
                <line x1="8" y1="11" x2="14" y2="11" />
              </svg>
            </div>
            <p className="font-mono text-sm text-[#64748b] mb-1">
              No problems match your filters
            </p>
            <p className="font-mono text-xs text-[#334155]">
              Try adjusting your search or clearing filters
            </p>
            <button
              onClick={() => {
                setSearch("");
                setActiveTopics(new Set());
                setActiveDifficulty("all");
              }}
              className="mt-4 font-mono text-[11px] px-4 py-1.5 border border-[#1e293b] text-[#64748b] rounded-sm hover:border-[#00e5a030] hover:text-[#00e5a0] transition-colors cursor-pointer"
            >
              Reset filters
            </button>
          </div>
        )}

        {problems.length === 0 && (
          <div className="text-center py-16">
            <div className="text-[#1e293b] mb-4">
              <svg
                width="40"
                height="40"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="mx-auto"
              >
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
            </div>
            <p className="font-mono text-sm text-[#64748b]">
              No problems found.
            </p>
            <p className="font-mono text-xs text-[#334155] mt-1">
              Add some to the database to get started.
            </p>
          </div>
        )}
      </div>
    </>
  );
}
