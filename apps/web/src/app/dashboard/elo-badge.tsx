"use client";

import { useState, useEffect } from "react";

function useSpinningNumber(target: number, duration = 1800) {
  const [value, setValue] = useState(0);

  useEffect(() => {
    const start = performance.now();
    let raf: number;

    function tick(now: number) {
      const elapsed = now - start;
      const progress = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      setValue(Math.round(eased * target));
      if (progress < 1) {
        raf = requestAnimationFrame(tick);
      }
    }

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target, duration]);

  return value;
}

export function EloBadge({ rating = 1200 }: { rating?: number }) {
  const displayValue = useSpinningNumber(rating);
  const [showInfo, setShowInfo] = useState(false);

  return (
    <div className="flex-shrink-0 relative">
      <div className="relative bg-[#0a0f1a] border border-[#ffffff10] rounded-sm overflow-hidden shadow-[0_0_40px_#ffffff08,0_0_80px_#ffffff04]">
        {/* Top accent line */}
        <div className="absolute top-0 left-0 right-0 h-[2px] bg-gradient-to-r from-transparent via-white/60 to-transparent" />

        {/* Main content */}
        <div className="px-4 py-3.5">
          <div className="flex items-center gap-2.5">
            {/* Trending icon */}
            <div className="w-9 h-9 flex items-center justify-center border border-[#1e293b] rounded-sm bg-[#060b14]">
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="white"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="float"
              >
                <polyline points="22 7 13.5 15.5 8.5 10.5 2 17" />
                <polyline points="16 7 22 7 22 13" />
              </svg>
            </div>

            {/* Rating display */}
            <div className="flex-1">
              <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#64748b] mb-1">
                Elo Rating
              </div>
              <div className="font-display text-3xl font-bold leading-none tabular-nums tracking-tight text-white"
                style={{
                  textShadow: "0 0 15px rgba(255,255,255,0.4), 0 0 40px rgba(255,255,255,0.15), 0 0 80px rgba(255,255,255,0.05)",
                }}
              >
                {displayValue}
              </div>
            </div>

            {/* Info toggle */}
            <button
              onClick={() => setShowInfo((prev) => !prev)}
              className="self-start mt-0.5 text-[#94a3b8] hover:text-white transition-colors cursor-pointer"
              aria-label="What contributes to your rating"
            >
              <svg
                width="15"
                height="15"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <circle cx="12" cy="12" r="10" />
                <path d="M12 16v-4" />
                <path d="M12 8h.01" />
              </svg>
            </button>
          </div>
        </div>

        {/* Info bar */}
        <div
          className={`overflow-hidden transition-[max-height] duration-300 ${
            showInfo ? "max-h-40" : "max-h-0"
          }`}
        >
          <div className="border-t border-[#1e293b] px-4 py-3 bg-[#060b14]">
            <div className="font-mono text-[9px] uppercase tracking-widest text-[#64748b] mb-2.5">
              What affects your rating
            </div>
            <div className="grid grid-cols-2 gap-x-4 gap-y-2">
              {[
                { label: "Problems solved", color: "#00e5a0" },
                { label: "Problem difficulty", color: "#00e5a0" },
                { label: "Hints used", color: "#f59e0b" },
                { label: "Time to solve", color: "#f59e0b" },
                { label: "Failed attempts", color: "#ef4444" },
                { label: "Consistency streak", color: "#7c3aed" },
              ].map((item) => (
                <div key={item.label} className="flex items-center gap-2">
                  <span
                    className="w-1.5 h-1.5 rounded-full flex-shrink-0"
                    style={{ backgroundColor: item.color }}
                  />
                  <span className="font-mono text-[10px] text-[#94a3b8]">
                    {item.label}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}
