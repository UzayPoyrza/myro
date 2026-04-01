"use client";

import { useState, useEffect } from "react";

export function ContestNavItem() {
  const [showToast, setShowToast] = useState(false);

  useEffect(() => {
    if (!showToast) return;
    const timer = setTimeout(() => setShowToast(false), 1500);
    return () => clearTimeout(timer);
  }, [showToast]);

  return (
    <button
      onClick={() => setShowToast(true)}
      className="relative hidden sm:inline font-mono text-xs text-[#ef4444]/50 hover:text-[#ef4444]/80 transition-colors cursor-default"
    >
      Contests

      {/* Toast popup */}
      <span
        className="absolute left-1/2 mt-2 px-3 py-1.5 bg-[#111827] border border-[#ef4444]/20 rounded-sm font-mono text-[10px] text-[#ef4444] whitespace-nowrap transition-all duration-200 pointer-events-none"
        style={{
          top: "100%",
          opacity: showToast ? 1 : 0,
          transform: showToast
            ? "translateX(-50%) translateY(0)"
            : "translateX(-50%) translateY(-4px)",
        }}
      >
        Coming soon
      </span>
    </button>
  );
}
