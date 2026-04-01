"use client";

import Link from "next/link";
import { useState, useCallback } from "react";
import { ContestNavItem } from "@/components/contest-nav-item";

/* ─── Copy Button ─── */

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [text]);

  return (
    <button
      onClick={handleCopy}
      className="flex-shrink-0 p-1.5 rounded-sm text-[#64748b] hover:text-[#f59e0b] transition-colors duration-200 cursor-pointer"
      aria-label="Copy to clipboard"
    >
      {copied ? (
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="#00e5a0"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polyline points="20 6 9 17 4 12" />
        </svg>
      ) : (
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
      )}
    </button>
  );
}

/* ─── Data ─── */

const QUICK_START_STEPS = [
  {
    number: "01",
    label: "Install",
    command: "cargo install myro",
    description: "Install the CLI from crates.io",
  },
  {
    number: "02",
    label: "Run",
    command: "myro",
    description: "Launch the TUI and start training",
  },
  {
    number: "03",
    label: "Update",
    command: "cargo install myro --force",
    description: "Update to the latest version",
  },
  {
    number: "04",
    label: "Uninstall",
    command: "cargo uninstall myro",
    description: "Remove Myro from your system",
  },
];

const COMMANDS = [
  { command: "myro", description: "Launch the interactive TUI trainer" },
  { command: "myro train", description: "Start a random training session" },
  {
    command: "myro train --topic dp",
    description: "Train on a specific topic",
  },
  { command: "myro stats", description: "View your rating and progress" },
  { command: "myro problems", description: "Browse the problem library" },
  { command: "myro config", description: "Open configuration settings" },
];

const CONFIG_TOML = `[general]
editor = "vim"
theme = "dark"

[training]
difficulty_range = [800, 1600]
preferred_topics = ["dp", "graphs", "greedy"]

[rating]
initial_elo = 1200`;

/* ─── Page Component ─── */

export default function CLIPage() {
  return (
    <div className="min-h-screen bg-[#030712] text-[#e2e8f0]">
      {/* ═══════════════ NAV ═══════════════ */}
      <header className="border-b border-[#1e293b] bg-[#030712] px-6 h-14 flex items-center justify-between">
        <div className="flex items-center gap-6">
          <Link href="/" className="flex items-center gap-1">
            <span className="font-display text-lg font-bold tracking-wide text-[#00e5a0] glow-text-strong">
              MYRO
            </span>
            <span className="font-display text-lg font-bold tracking-wide text-[#e2e8f0]">
              Web
            </span>
          </Link>
          <div className="hidden sm:flex items-center gap-1">
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="#334155"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <polyline points="9 18 15 12 9 6" />
            </svg>
            <span className="font-mono text-xs text-[#f59e0b]">CLI</span>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <Link
            href="/dashboard"
            className="font-mono text-xs text-[#e2e8f0] hover:text-[#00e5a0] transition-colors hidden sm:inline"
          >
            Dashboard
          </Link>
          <Link
            href="/problems"
            className="font-mono text-xs text-[#e2e8f0] hover:text-[#00e5a0] transition-colors hidden sm:inline"
          >
            Problems
          </Link>
          <ContestNavItem />
          <Link href="/sign-in" className="font-mono text-xs px-4 py-1.5 border border-[#1e293b] rounded-sm text-[#e2e8f0] hover:border-[#00e5a0] hover:text-[#00e5a0] transition-all duration-200">
            Sign in
          </Link>
        </div>
      </header>

      <main className="max-w-4xl mx-auto px-6 py-8">
        {/* ═══════════════ HERO ═══════════════ */}
        <section className="fade-in mb-10">
          <div className="mb-5">
            <div className="flex items-center gap-4 mb-3">
              <div className="w-10 h-10 flex items-center justify-center border border-[#f59e0b]/30 rounded-sm bg-[#f59e0b]/5 flex-shrink-0">
                <svg
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#f59e0b"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <polyline points="4 17 10 11 4 5" />
                  <line x1="12" y1="19" x2="20" y2="19" />
                </svg>
              </div>
              <h1 className="font-display text-3xl sm:text-4xl font-bold text-white tracking-tight">
                Myro{" "}
                <span className="text-[#f59e0b] drop-shadow-[0_0_20px_#f59e0b30]">
                  CLI
                </span>
              </h1>
            </div>
            <p className="font-mono text-sm text-[#94a3b8] max-w-xl leading-relaxed">
              The full competitive programming experience, right in your
              terminal.
            </p>
          </div>

          {/* Install command box */}
          <div className="group relative bg-[#060b14] border border-[#1e293b] rounded-sm p-4 flex items-center justify-between gap-4 max-w-lg transition-all duration-300 hover:border-[#f59e0b]/30 hover:shadow-[0_0_30px_#f59e0b08]">
            <div className="flex items-center gap-3 font-mono text-sm overflow-x-auto">
              <span className="text-[#f59e0b] flex-shrink-0">$</span>
              <span className="text-[#e2e8f0]">cargo install myro</span>
            </div>
            <CopyButton text="cargo install myro" />
          </div>
        </section>

        {/* ═══════════════ QUICK START ═══════════════ */}
        <section className="fade-in-delay-1 mb-10">
          <div className="flex items-center gap-3 mb-4">
            <h2 className="font-mono text-xs text-[#94a3b8] uppercase tracking-widest">
              // getting started
            </h2>
            <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
          </div>

          <div className="space-y-3">
            {QUICK_START_STEPS.map((step) => (
              <div
                key={step.number}
                className="group bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-4 sm:p-5 flex flex-col sm:flex-row sm:items-center gap-3 sm:gap-5 transition-all duration-300 hover:border-[#f59e0b]/20 hover:bg-[#0d1220]"
              >
                {/* Step number */}
                <div className="flex items-center gap-3 sm:gap-5 flex-shrink-0">
                  <span className="font-display text-lg font-bold text-[#f59e0b] w-7">
                    {step.number}
                  </span>
                  <span className="font-display text-sm font-semibold text-white w-20">
                    {step.label}
                  </span>
                </div>

                {/* Command block */}
                <div className="flex items-center gap-2 bg-[#060b14] border border-[#1e293b] rounded-sm px-3 py-2 flex-shrink-0 transition-all duration-300 group-hover:border-[#f59e0b]/20">
                  <span className="text-[#f59e0b] font-mono text-xs flex-shrink-0">
                    $
                  </span>
                  <code className="font-mono text-xs text-[#e2e8f0] whitespace-nowrap">
                    {step.command}
                  </code>
                  <CopyButton text={step.command} />
                </div>

                {/* Description */}
                <p className="font-mono text-xs text-[#64748b] sm:ml-auto">
                  {step.description}
                </p>
              </div>
            ))}
          </div>
        </section>

        {/* ═══════════════ COMMANDS ═══════════════ */}
        <section className="fade-in-delay-2 mb-10">
          <div className="flex items-center gap-3 mb-4">
            <h2 className="font-mono text-xs text-[#94a3b8] uppercase tracking-widest">
              // commands
            </h2>
            <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
          </div>

          <div className="bg-[#0a0f1a] border border-[#1e293b] rounded-sm overflow-hidden">
            {COMMANDS.map((cmd, i) => (
              <div
                key={cmd.command}
                className={`group flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-6 px-5 py-4 transition-all duration-200 hover:bg-[#0d1220] ${
                  i < COMMANDS.length - 1 ? "border-b border-[#1e293b]/60" : ""
                }`}
              >
                <div className="flex items-center gap-2 flex-shrink-0 min-w-0 sm:min-w-[280px]">
                  <code className="font-mono text-xs text-[#f59e0b] whitespace-nowrap">
                    {cmd.command}
                  </code>
                </div>
                <span className="hidden sm:inline text-[#334155] font-mono text-xs flex-shrink-0">
                  --
                </span>
                <p className="font-mono text-xs text-[#94a3b8]">
                  {cmd.description}
                </p>
              </div>
            ))}
          </div>
        </section>

        {/* ═══════════════ CONFIGURATION ═══════════════ */}
        <section className="fade-in-delay-3 mb-10">
          <div className="flex items-center gap-3 mb-4">
            <h2 className="font-mono text-xs text-[#94a3b8] uppercase tracking-widest">
              // configuration
            </h2>
            <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
          </div>

          <div className="mb-4">
            <p className="font-mono text-sm text-[#94a3b8] mb-2">
              Configuration lives at:
            </p>
            <div className="inline-flex items-center gap-2 bg-[#060b14] border border-[#1e293b] rounded-sm px-3 py-2">
              <code className="font-mono text-xs text-[#e2e8f0]">
                ~/.config/myro/config.toml
              </code>
              <CopyButton text="~/.config/myro/config.toml" />
            </div>
          </div>

          {/* TOML code block */}
          <div className="relative bg-[#060b14] border border-[#1e293b] rounded-sm overflow-hidden">
            {/* Header bar */}
            <div className="flex items-center justify-between px-4 py-2.5 border-b border-[#1e293b]/60 bg-[#0a0f1a]/50">
              <div className="flex items-center gap-2">
                <div className="w-2.5 h-2.5 rounded-full bg-[#334155]" />
                <div className="w-2.5 h-2.5 rounded-full bg-[#334155]" />
                <div className="w-2.5 h-2.5 rounded-full bg-[#334155]" />
              </div>
              <span className="font-mono text-[10px] text-[#334155]">
                config.toml
              </span>
              <CopyButton text={CONFIG_TOML} />
            </div>

            {/* Code content */}
            <div className="p-4 overflow-x-auto">
              <pre className="font-mono text-xs leading-relaxed">
                <span className="text-[#64748b]">[general]</span>
                {"\n"}
                <span className="text-[#94a3b8]">editor</span>
                <span className="text-[#334155]"> = </span>
                <span className="text-[#00e5a0]">&quot;vim&quot;</span>
                {"\n"}
                <span className="text-[#94a3b8]">theme</span>
                <span className="text-[#334155]"> = </span>
                <span className="text-[#00e5a0]">&quot;dark&quot;</span>
                {"\n\n"}
                <span className="text-[#64748b]">[training]</span>
                {"\n"}
                <span className="text-[#94a3b8]">difficulty_range</span>
                <span className="text-[#334155]"> = </span>
                <span className="text-[#f59e0b]">[800, 1600]</span>
                {"\n"}
                <span className="text-[#94a3b8]">preferred_topics</span>
                <span className="text-[#334155]"> = </span>
                <span className="text-[#f59e0b]">[</span>
                <span className="text-[#00e5a0]">
                  &quot;dp&quot;
                </span>
                <span className="text-[#f59e0b]">, </span>
                <span className="text-[#00e5a0]">
                  &quot;graphs&quot;
                </span>
                <span className="text-[#f59e0b]">, </span>
                <span className="text-[#00e5a0]">
                  &quot;greedy&quot;
                </span>
                <span className="text-[#f59e0b]">]</span>
                {"\n\n"}
                <span className="text-[#64748b]">[rating]</span>
                {"\n"}
                <span className="text-[#94a3b8]">initial_elo</span>
                <span className="text-[#334155]"> = </span>
                <span className="text-[#f59e0b]">1200</span>
              </pre>
            </div>
          </div>
        </section>

        {/* ═══════════════ DATA & SYNC ═══════════════ */}
        <section className="fade-in-delay-4 mb-10">
          <div className="flex items-center gap-3 mb-4">
            <h2 className="font-mono text-xs text-[#94a3b8] uppercase tracking-widest">
              // data
            </h2>
            <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
          </div>

          <div className="bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-6">
            <div className="flex items-start gap-4">
              <div className="flex-shrink-0 mt-0.5 text-[#f59e0b]">
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <ellipse cx="12" cy="5" rx="9" ry="3" />
                  <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
                  <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
                </svg>
              </div>
              <div>
                <p className="font-mono text-sm text-[#e2e8f0] mb-3">
                  All data is stored locally in a single SQLite database:
                </p>
                <div className="inline-flex items-center gap-2 bg-[#060b14] border border-[#1e293b] rounded-sm px-3 py-2 mb-4">
                  <code className="font-mono text-xs text-[#e2e8f0]">
                    ~/.local/share/myro/myro.db
                  </code>
                  <CopyButton text="~/.local/share/myro/myro.db" />
                </div>
                <div className="space-y-2">
                  <p className="font-mono text-xs text-[#94a3b8] leading-relaxed">
                    Problems, submissions, ratings, and skill levels are all
                    persisted here. The database is shared between Myro Web and
                    the CLI, so your progress syncs automatically.
                  </p>
                  <p className="font-mono text-xs text-[#64748b] leading-relaxed">
                    SQLite WAL mode is enabled for safe concurrent access from
                    multiple processes.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* ═══════════════ FOOTER ═══════════════ */}
        <footer className="fade-in-delay-5 text-center pt-8 pb-6 border-t border-[#1e293b]/40">
          <p className="font-mono text-xs text-[#334155]">
            myro v0.1.0 &middot; open source &middot; built for competitive
            programmers
          </p>
        </footer>
      </main>
    </div>
  );
}
