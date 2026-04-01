import Link from "next/link";
import { prisma } from "@/lib/db";
import { QuickStartCard } from "./quick-start";
import { EloBadge } from "./elo-badge";
import { EloGraph } from "./elo-graph";
import { ContestNavItem } from "@/components/contest-nav-item";

export const dynamic = "force-dynamic";

export default async function DashboardPage() {
  const problems = await prisma.problem.findMany({ select: { id: true } });
  const problemIds = problems.map((p) => p.id);

  return (
    <div className="min-h-screen bg-[#030712] text-[#e2e8f0]">
      {/* Top bar */}
      <header className="border-b border-[#1e293b] bg-[#030712] px-6 h-14 flex items-center justify-between">
        <Link href="/" className="flex items-center gap-1">
          <span className="font-display text-lg font-bold tracking-wide text-[#00e5a0] glow-text-strong">
            MYRO
          </span>
          <span className="font-display text-lg font-bold tracking-wide text-[#e2e8f0]">
            Web
          </span>
        </Link>
        <div className="flex items-center gap-4">
          <Link
            href="/problems"
            className="font-mono text-xs text-[#e2e8f0] hover:text-[#00e5a0] transition-colors hidden sm:inline"
          >
            Problems
          </Link>
          <Link
            href="/cli"
            className="font-mono text-xs text-[#e2e8f0] hover:text-[#00e5a0] transition-colors hidden sm:inline"
          >
            CLI
          </Link>
          <ContestNavItem />
          <Link href="/sign-in" className="font-mono text-xs px-4 py-1.5 border border-[#1e293b] rounded-sm text-[#e2e8f0] hover:border-[#00e5a0] hover:text-[#00e5a0] transition-all duration-200">
            Sign in
          </Link>
        </div>
      </header>

      <main className="max-w-5xl mx-auto px-6 py-10">
        {/* Greeting + ELO */}
        <div className="fade-in mb-10 flex items-start justify-between gap-6">
          <div>
            <div className="flex items-center gap-3 mb-1">
              <h1 className="font-display text-2xl sm:text-3xl font-bold text-white tracking-tight">
                Welcome back
              </h1>
              <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent hidden sm:block" />
            </div>
            <p className="text-[#94a3b8] font-mono text-sm">
              Choose your path, commander.
            </p>
          </div>

          {/* ELO Rating Badge */}
          <EloBadge rating={1200} />
        </div>

        {/* Action cards — Quick Start dominant */}
        <div className="grid grid-cols-1 md:grid-cols-[1.3fr_1fr_1fr] gap-4 mb-12">

          {/* Card 1 — Quick Start (RECOMMENDED) */}
          <QuickStartCard problemIds={problemIds} />

          {/* Card 2 — Browse Library */}
          <Link
            href="/problems"
            className="fade-in-delay-2 group relative block bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-6 transition-all duration-300 hover:scale-[1.02] hover:border-[#e2e8f0]/20 hover:bg-[#111827] hover:shadow-[0_0_30px_#e2e8f010]"
          >
            <div className="mb-4 text-[#e2e8f0]">
              <svg
                width="28"
                height="28"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <rect x="3" y="3" width="7" height="7" />
                <rect x="14" y="3" width="7" height="7" />
                <rect x="3" y="14" width="7" height="7" />
                <rect x="14" y="14" width="7" height="7" />
              </svg>
            </div>
            <h2 className="font-display text-lg font-bold text-white mb-2">
              Browse Library
            </h2>
            <p className="font-mono text-xs text-[#cbd5e1] leading-relaxed">
              Explore all problems by topic and difficulty
            </p>
            <div className="mt-4 font-mono text-[11px] text-[#e2e8f0] flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-all duration-300 group-hover:translate-x-1">
              <span>browse all</span>
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <line x1="5" y1="12" x2="19" y2="12" />
                <polyline points="12 5 19 12 12 19" />
              </svg>
            </div>
          </Link>

          {/* Card 3 — Enter Contest (coming soon, RED) */}
          <div className="fade-in-delay-3 group relative bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-6 cursor-not-allowed overflow-hidden">
            {/* Red locked overlay */}
            <div className="absolute inset-0 bg-[#ef444408] pointer-events-none" />

            {/* Status indicator */}
            <div className="flex items-center gap-2 mb-4">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#ef4444] opacity-50" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-[#ef4444]" />
              </span>
              <span className="font-mono text-[10px] uppercase tracking-widest text-[#ef4444]">
                Coming Soon
              </span>
            </div>

            <div className="mb-4 text-[#ef4444]/40">
              <svg
                width="28"
                height="28"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M6 9H4.5a2.5 2.5 0 0 1 0-5C7 4 6 2 12 2s5 2 7.5 2a2.5 2.5 0 0 1 0 5H18" />
                <path d="M18 9v8a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V9" />
                <path d="M12 9v4" />
              </svg>
            </div>
            <h2 className="font-display text-lg font-bold text-[#ef4444]/50 mb-2">
              Enter Contest
            </h2>
            <p className="font-mono text-xs text-[#cbd5e1]/30 leading-relaxed">
              Compete in timed problem sets against the clock
            </p>

            {/* Lock icon */}
            <div className="mt-4 flex items-center gap-1.5">
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#ef4444"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="opacity-40"
              >
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
              <span className="font-mono text-[10px] text-[#ef4444]/40">
                locked
              </span>
            </div>
          </div>
        </div>

        {/* Stats section */}
        <div className="fade-in-delay-3 mb-12">
          <div className="flex items-center gap-3 mb-4">
            <h3 className="font-mono text-xs text-[#94a3b8] uppercase tracking-widest">
              // your stats
            </h3>
            <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
            <div className="bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-4 transition-all duration-300 hover:border-[#1e293b80] hover:bg-[#0d1220]">
              <div className="font-mono text-[11px] text-[#94a3b8] mb-1.5">
                Problems Solved
              </div>
              <div className="font-display text-2xl font-bold text-white">
                0
              </div>
            </div>
            <div className="bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-4 transition-all duration-300 hover:border-[#1e293b80] hover:bg-[#0d1220]">
              <div className="font-mono text-[11px] text-[#94a3b8] mb-1.5">
                Current Streak
              </div>
              <div className="font-display text-2xl font-bold text-white">
                0<span className="text-sm font-normal text-[#94a3b8] ml-1">d</span>
              </div>
            </div>
            <div className="bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-4 transition-all duration-300 hover:border-[#1e293b80] hover:bg-[#0d1220]">
              <div className="font-mono text-[11px] text-[#94a3b8] mb-1.5">
                Observations Found
              </div>
              <div className="font-display text-2xl font-bold text-white">
                0
              </div>
            </div>
            <div className="bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-4 transition-all duration-300 hover:border-[#1e293b80] hover:bg-[#0d1220]">
              <div className="font-mono text-[11px] text-[#94a3b8] mb-1.5">
                Avg. Hints Used
              </div>
              <div className="font-display text-2xl font-bold text-white">
                --
              </div>
            </div>
          </div>
        </div>

        {/* ELO Progress Graph */}
        <div className="fade-in-delay-3 mb-12">
          <div className="flex items-center gap-3 mb-4">
            <h3 className="font-mono text-xs text-[#94a3b8] uppercase tracking-widest">
              // rating progress
            </h3>
            <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
          </div>
          <EloGraph />
        </div>

        {/* Recent activity (empty state) */}
        <div className="fade-in-delay-4 mb-12">
          <div className="flex items-center gap-3 mb-4">
            <h3 className="font-mono text-xs text-[#94a3b8] uppercase tracking-widest">
              // recent sessions
            </h3>
            <div className="h-[1px] flex-1 bg-gradient-to-r from-[#1e293b] to-transparent" />
          </div>
          <div className="bg-[#0a0f1a] border border-[#1e293b] border-dashed rounded-sm p-8 text-center">
            <div className="text-[#334155] mb-3">
              <svg
                width="32"
                height="32"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="mx-auto"
              >
                <circle cx="12" cy="12" r="10" />
                <polyline points="12 6 12 12 16 14" />
              </svg>
            </div>
            <p className="font-mono text-sm text-[#64748b]">
              No sessions yet.
            </p>
            <p className="font-mono text-xs text-[#334155] mt-1">
              Start a problem to see your history here.
            </p>
          </div>
        </div>

        {/* Footer */}
        <footer className="fade-in-delay-5 text-center pb-6">
          <p className="font-mono text-xs text-[#334155]">
            myro v0.1.0 &middot; open source &middot; built for competitive programmers
          </p>
        </footer>
      </main>
    </div>
  );
}
