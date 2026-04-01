"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";

const LOADING_STEPS = [
  { text: "Evaluating your strengths...", duration: 1200 },
  { text: "Identifying weak areas...", duration: 1000 },
  { text: "Running recommendation algorithm...", duration: 1400 },
  { text: "Selecting optimal problem...", duration: 800 },
  { text: "Preparing coaching session...", duration: 600 },
];

export function QuickStartCard({ problemIds }: { problemIds: string[] }) {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [stepIndex, setStepIndex] = useState(0);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    if (!loading) return;

    if (stepIndex >= LOADING_STEPS.length) {
      // Pick random problem and navigate
      const randomId =
        problemIds[Math.floor(Math.random() * problemIds.length)];
      router.push(`/problems/${randomId}/start`);
      return;
    }

    const step = LOADING_STEPS[stepIndex];
    const timer = setTimeout(() => {
      setStepIndex((prev) => prev + 1);
      setProgress(((stepIndex + 1) / LOADING_STEPS.length) * 100);
    }, step.duration);

    return () => clearTimeout(timer);
  }, [loading, stepIndex, problemIds, router]);

  function handleClick(e: React.MouseEvent) {
    e.preventDefault();
    setStepIndex(0);
    setProgress(0);
    setLoading(true);
  }

  return (
    <>
      {/* Quick Start Card */}
      <button
        onClick={handleClick}
        className="fade-in-delay-1 group relative block w-full text-left bg-gradient-to-b from-[#00e5a010] to-[#0a0f1a] border-2 border-breathe rounded-sm p-6 transition-all duration-300 hover:scale-[1.02] hover:border-[#00e5a0] hover:shadow-[0_0_40px_#00e5a020,0_0_100px_#00e5a010] pulse-glow cursor-pointer"
      >
        {/* Top accent bar */}
        <div className="absolute top-0 left-0 right-0 h-[2px] bg-gradient-to-r from-transparent via-[#00e5a0] to-transparent opacity-70 group-hover:opacity-100 transition-opacity" />

        {/* Recommended badge */}
        <div className="flex items-center gap-2 mb-4">
          <span className="relative flex h-2 w-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#00e5a0] opacity-75" />
            <span className="relative inline-flex rounded-full h-2 w-2 bg-[#00e5a0]" />
          </span>
          <span className="font-mono text-[10px] uppercase tracking-widest text-[#00e5a0]">
            Recommended
          </span>
        </div>

        <div className="mb-4 text-[#00e5a0]">
          <svg
            width="32"
            height="32"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="float"
          >
            <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />
          </svg>
        </div>
        <h2 className="font-display text-xl font-bold text-[#00e5a0] glow-text mb-2">
          Quick Start
        </h2>
        <p className="font-mono text-xs text-[#cbd5e1] leading-relaxed mb-5">
          Jump into a recommended problem matched to your level. The coach will
          guide you through observations step by step.
        </p>

        {/* CTA button */}
        <div className="inline-flex items-center gap-2 px-5 py-2 bg-[#00e5a0] rounded-sm font-mono text-xs font-semibold text-[#030712] group-hover:bg-[#00cc8e] group-hover:shadow-[0_0_20px_#00e5a030] transition-all duration-300">
          <span>Start Training</span>
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="group-hover:translate-x-0.5 transition-transform"
          >
            <line x1="5" y1="12" x2="19" y2="12" />
            <polyline points="12 5 19 12 12 19" />
          </svg>
        </div>
      </button>

      {/* Loading Modal Overlay */}
      {loading && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center">
          {/* Backdrop */}
          <div className="absolute inset-0 bg-[#030712]/90 backdrop-blur-sm" />

          {/* Modal */}
          <div className="relative z-10 w-full max-w-md mx-4 bg-[#0a0f1a] border border-[#1e293b] rounded-sm p-8 shadow-[0_0_60px_#00e5a010]">
            {/* Top accent */}
            <div className="absolute top-0 left-0 right-0 h-[2px] bg-gradient-to-r from-transparent via-[#00e5a0] to-transparent" />

            {/* Icon */}
            <div className="flex justify-center mb-6">
              <div className="w-14 h-14 flex items-center justify-center border border-[#00e5a0]/30 rounded-sm">
                <svg
                  width="28"
                  height="28"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#00e5a0"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="animate-pulse"
                >
                  <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />
                </svg>
              </div>
            </div>

            {/* Title */}
            <h3 className="font-display text-lg font-bold text-white text-center mb-1">
              Finding your next challenge
            </h3>
            <p className="font-mono text-[11px] text-[#94a3b8] text-center mb-8">
              Analyzing your profile to select the best problem...
            </p>

            {/* Steps log */}
            <div className="space-y-3 mb-8">
              {LOADING_STEPS.map((step, i) => {
                const isDone = i < stepIndex;
                const isCurrent = i === stepIndex;

                return (
                  <div
                    key={i}
                    className={`flex items-center gap-3 transition-all duration-300 ${
                      isDone
                        ? "opacity-100"
                        : isCurrent
                          ? "opacity-100"
                          : "opacity-20"
                    }`}
                  >
                    {/* Status indicator */}
                    <div className="w-5 h-5 flex items-center justify-center flex-shrink-0">
                      {isDone ? (
                        <svg
                          width="14"
                          height="14"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="#00e5a0"
                          strokeWidth="2.5"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <polyline points="20 6 9 17 4 12" />
                        </svg>
                      ) : isCurrent ? (
                        <div className="w-3 h-3 border-2 border-[#00e5a0] border-t-transparent rounded-full animate-spin" />
                      ) : (
                        <div className="w-2 h-2 rounded-full bg-[#1e293b]" />
                      )}
                    </div>

                    {/* Text */}
                    <span
                      className={`font-mono text-xs ${
                        isDone
                          ? "text-[#00e5a0]"
                          : isCurrent
                            ? "text-white"
                            : "text-[#334155]"
                      }`}
                    >
                      {step.text}
                    </span>
                  </div>
                );
              })}
            </div>

            {/* Progress bar */}
            <div className="h-1 bg-[#1e293b] rounded-full overflow-hidden">
              <div
                className="h-full bg-[#00e5a0] rounded-full transition-all duration-500 ease-out shadow-[0_0_10px_#00e5a040]"
                style={{ width: `${progress}%` }}
              />
            </div>
          </div>
        </div>
      )}
    </>
  );
}
