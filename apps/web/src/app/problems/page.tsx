import Link from "next/link";
import { prisma } from "@/lib/db";
import { ProblemsList } from "./problems-list";
import { ContestNavItem } from "@/components/contest-nav-item";

export const dynamic = "force-dynamic";

export default async function ProblemsPage() {
  const problems = await prisma.problem.findMany({
    include: {
      routes: {
        include: { observations: true },
      },
    },
    orderBy: { difficulty: "asc" },
  });

  const serialized = problems.map((p) => ({
    id: p.id,
    title: p.title,
    difficulty: p.difficulty,
    topic: p.topic,
    description: p.description,
    routeCount: p.routes.length,
    totalObs: p.routes.reduce((sum, r) => sum + r.observations.length, 0),
  }));

  return (
    <div className="min-h-screen bg-[#030712] text-[#e2e8f0]">
      {/* Top bar — matches dashboard */}
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
            <span className="font-mono text-xs text-[#94a3b8]">
              Problems
            </span>
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

      <main className="max-w-5xl mx-auto px-6 py-8">
        {/* Header */}
        <div className="fade-in mb-8">
          <div className="flex items-center justify-between mb-1">
            <h1 className="font-display text-2xl sm:text-3xl font-bold text-white tracking-tight">
              Problem Library
            </h1>
            <div className="flex items-center gap-4">
              {/* Difficulty distribution mini-bar */}
              <div className="hidden sm:flex items-center gap-1 font-mono text-[10px]">
                <span className="text-[#00e5a0]">
                  {serialized.filter((p) => p.difficulty <= 1200).length} easy
                </span>
                <span className="text-[#334155]">/</span>
                <span className="text-[#f59e0b]">
                  {serialized.filter((p) => p.difficulty > 1200 && p.difficulty <= 1500).length} med
                </span>
                <span className="text-[#334155]">/</span>
                <span className="text-[#ef4444]">
                  {serialized.filter((p) => p.difficulty > 1500).length} hard
                </span>
              </div>
            </div>
          </div>
          <p className="text-[#64748b] font-mono text-xs">
            Search, filter, and start training on any problem
          </p>
        </div>

        <ProblemsList problems={serialized} />
      </main>
    </div>
  );
}
