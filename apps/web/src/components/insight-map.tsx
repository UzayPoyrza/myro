"use client";

import { Check, Lock, Lightbulb, Shuffle } from "lucide-react";

interface Observation {
  id: string;
  order: number;
  title: string;
  isUnlocked: boolean;
  hintLevel: number;
}

interface InsightMapProps {
  observations: Observation[];
  totalObservations: number;
  unlockedCount: number;
  routeName: string;
  onMoreHint: () => void;
  onSwitchRoute?: () => void;
}

const MAX_HINT_LEVEL = 3;

function HintDots({ level }: { level: number }) {
  return (
    <div className="flex items-center gap-0.5" aria-label={`Hint level ${level} of ${MAX_HINT_LEVEL}`}>
      {Array.from({ length: MAX_HINT_LEVEL }).map((_, i) => (
        <span
          key={i}
          className={`text-[11px] leading-none select-none ${
            i < level ? "text-teal-400" : "text-zinc-600"
          }`}
        >
          {i < level ? "●" : "○"}
        </span>
      ))}
    </div>
  );
}

export function InsightMap({
  observations,
  totalObservations,
  unlockedCount,
  routeName,
  onMoreHint,
  onSwitchRoute,
}: InsightMapProps) {
  const progressPct =
    totalObservations > 0
      ? Math.round((unlockedCount / totalObservations) * 100)
      : 0;

  const sorted = [...observations].sort((a, b) => a.order - b.order);

  return (
    <div className="flex flex-col bg-zinc-900 border border-zinc-700/50 rounded-md overflow-hidden">
      {/* Header */}
      <div className="px-4 py-3 border-b border-zinc-700/50 bg-zinc-800/40 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <Lightbulb className="w-3.5 h-3.5 text-amber-400 flex-shrink-0" />
          <span className="font-mono text-xs font-semibold uppercase tracking-widest text-zinc-400 truncate">
            Insight Map
          </span>
        </div>
        <span className="font-mono text-xs text-zinc-500 flex-shrink-0 truncate max-w-[50%] text-right">
          {routeName}
        </span>
      </div>

      {/* Progress bar */}
      <div className="px-4 pt-3 pb-1">
        <div className="flex items-center justify-between mb-1.5">
          <span className="font-mono text-xs text-zinc-500">
            Progress
          </span>
          <span className="font-mono text-xs text-zinc-400">
            {unlockedCount}
            <span className="text-zinc-600"> / </span>
            {totalObservations}
          </span>
        </div>
        <div className="h-1.5 w-full bg-zinc-800 rounded-full overflow-hidden border border-zinc-700/40">
          <div
            className="h-full bg-teal-500 rounded-full transition-all duration-500"
            style={{ width: `${progressPct}%` }}
            role="progressbar"
            aria-valuenow={unlockedCount}
            aria-valuemin={0}
            aria-valuemax={totalObservations}
          />
        </div>
        <span className="font-mono text-[10px] text-zinc-600 mt-1 block">
          {progressPct}% complete
        </span>
      </div>

      {/* Observation list */}
      <div className="flex-1 overflow-y-auto px-4 py-2 flex flex-col gap-1.5">
        {sorted.map((obs) => (
          <div
            key={obs.id}
            className={`flex items-center gap-3 px-3 py-2 rounded border transition-colors ${
              obs.isUnlocked
                ? "bg-zinc-800/50 border-zinc-700/40"
                : "bg-zinc-900/60 border-zinc-800/50 opacity-70"
            }`}
          >
            {/* Icon */}
            <div className="flex-shrink-0">
              {obs.isUnlocked ? (
                <div className="w-5 h-5 rounded-full bg-teal-500/20 border border-teal-500/40 flex items-center justify-center">
                  <Check className="w-3 h-3 text-teal-400" strokeWidth={2.5} />
                </div>
              ) : (
                <div className="w-5 h-5 rounded-full bg-zinc-800 border border-zinc-700/50 flex items-center justify-center">
                  <Lock className="w-2.5 h-2.5 text-zinc-600" strokeWidth={2} />
                </div>
              )}
            </div>

            {/* Label */}
            <span
              className={`flex-1 font-mono text-xs leading-snug ${
                obs.isUnlocked ? "text-zinc-200" : "text-zinc-600"
              }`}
            >
              {obs.isUnlocked ? obs.title : `Insight #${obs.order}`}
            </span>

            {/* Hint dots (only for unlocked observations that have hintLevel > 0, or all unlocked) */}
            {obs.isUnlocked && (
              <div className="flex-shrink-0">
                <HintDots level={obs.hintLevel} />
              </div>
            )}
          </div>
        ))}

        {sorted.length === 0 && (
          <p className="font-mono text-xs text-zinc-600 text-center py-4">
            No observations yet.
          </p>
        )}
      </div>

      {/* Actions */}
      <div className="px-4 py-3 border-t border-zinc-700/50 flex gap-2">
        <button
          type="button"
          onClick={onMoreHint}
          className="flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded bg-teal-600/20 hover:bg-teal-600/30 border border-teal-500/30 hover:border-teal-500/50 text-teal-300 font-mono text-xs font-medium transition-colors"
        >
          <Lightbulb className="w-3.5 h-3.5" />
          More Hint
        </button>

        {onSwitchRoute && (
          <button
            type="button"
            onClick={onSwitchRoute}
            className="flex items-center justify-center gap-1.5 px-3 py-2 rounded bg-zinc-800/60 hover:bg-zinc-700/60 border border-zinc-700/50 hover:border-zinc-600/50 text-zinc-400 hover:text-zinc-200 font-mono text-xs font-medium transition-colors"
          >
            <Shuffle className="w-3.5 h-3.5" />
            Switch Route
          </button>
        )}
      </div>
    </div>
  );
}
