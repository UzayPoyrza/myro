"use client";

import Link from "next/link";
import { useState, useEffect, useRef } from "react";
import { ContestNavItem } from "@/components/contest-nav-item";

const CYCLING_WORDS = ["LeetCode", "Codeforces", "competitive programming"];
const TYPE_SPEED = 60;
const DELETE_SPEED = 35;
const PAUSE_BEFORE_DELETE = 1200;

function useTypingCycle(words: string[]) {
  const [displayed, setDisplayed] = useState("");
  const [wordIndex, setWordIndex] = useState(0);
  const [isDeleting, setIsDeleting] = useState(false);

  useEffect(() => {
    const current = words[wordIndex];

    if (!isDeleting && displayed === current) {
      const timeout = setTimeout(
        () => setIsDeleting(true),
        PAUSE_BEFORE_DELETE
      );
      return () => clearTimeout(timeout);
    }

    if (isDeleting && displayed === "") {
      setIsDeleting(false);
      setWordIndex((prev) => (prev + 1) % words.length);
      return;
    }

    const speed = isDeleting ? DELETE_SPEED : TYPE_SPEED;
    const timeout = setTimeout(() => {
      if (isDeleting) {
        setDisplayed(current.slice(0, displayed.length - 1));
      } else {
        setDisplayed(current.slice(0, displayed.length + 1));
      }
    }, speed);

    return () => clearTimeout(timeout);
  }, [displayed, isDeleting, wordIndex, words]);

  return displayed;
}

function useInView(threshold = 0.15) {
  const ref = useRef<HTMLDivElement>(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setInView(true);
          observer.unobserve(el);
        }
      },
      { threshold }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [threshold]);

  return { ref, inView };
}

/* ─── Steps Data ─── */

const STEPS = [
  {
    number: "01",
    title: "Observe",
    description:
      "Each problem has key observations — insights you must discover to solve it. The coach guides you toward them without giving answers.",
  },
  {
    number: "02",
    title: "Unlock",
    description:
      "As you think through the problem, the AI detects your insights and unlocks observations on your insight map in real time.",
  },
  {
    number: "03",
    title: "Implement",
    description:
      "Once all observations are found, switch to implementation mode with a structured checklist and built-in code editor.",
  },
];

/* ─── Page Component ─── */

export default function LandingPage() {
  const typedText = useTypingCycle(CYCLING_WORDS);
  const howItWorks = useInView(0.12);
  const whyMyro = useInView(0.15);
  const [activeStep, setActiveStep] = useState(-1);
  const [activeCard, setActiveCard] = useState(-1);

  // Sequential step activation when "how it works" comes into view
  useEffect(() => {
    if (!howItWorks.inView) return;

    const timers = [
      setTimeout(() => setActiveStep(0), 600),
      setTimeout(() => setActiveStep(1), 1400),
      setTimeout(() => setActiveStep(2), 2200),
    ];

    return () => timers.forEach(clearTimeout);
  }, [howItWorks.inView]);

  // Sequential card reveal for "Why Myro" section
  useEffect(() => {
    if (!whyMyro.inView) return;

    const timers = [
      setTimeout(() => setActiveCard(0), 300),
      setTimeout(() => setActiveCard(1), 600),
      setTimeout(() => setActiveCard(2), 900),
      setTimeout(() => setActiveCard(3), 1200),
    ];

    return () => timers.forEach(clearTimeout);
  }, [whyMyro.inView]);

  return (
    <div className="relative">
      {/* Ambient effects */}
      <div className="hero-glow" />

      {/* ═══════════════ NAV ═══════════════ */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-[#030712] border-b border-[#1e293b]/60">
        <div className="max-w-6xl mx-auto px-6 h-14 flex items-center justify-between">
          <Link href="/" className="flex items-center gap-1 slide-in-left">
            <span className="font-display text-lg font-bold tracking-wide text-[#00e5a0] glow-text-strong">
              MYRO
            </span>
            <span className="font-display text-lg font-bold tracking-wide text-[#e2e8f0]">
              Web
            </span>
          </Link>

          <div className="flex items-center gap-5 fade-in">
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
        </div>
      </nav>

      {/* ═══════════════ HERO ═══════════════ */}
      <section className="relative h-dvh flex flex-col items-center px-4 pt-14">
        <div className="relative z-10 flex flex-col items-center flex-1 w-full max-w-5xl mx-auto pt-[8vh] sm:pt-[10vh]">
          {/* Headline */}
          <div className="text-center mb-6">
            <h1 className="font-display text-2xl sm:text-3xl md:text-4xl font-bold leading-tight fade-in">
              <span className="shimmer-text">
                The fastest way to get good at
              </span>
              <br />
              <span className="text-[#00e5a0] glow-text-strong whitespace-nowrap">
                {typedText}
                <span className="cursor-blink text-[#00e5a0]">_</span>
              </span>
            </h1>
            <p className="mt-3 text-xs sm:text-sm text-[#cbd5e1] font-mono fade-in-delay-1">
              AI-powered observation coaching for competitive programmers.
            </p>
          </div>

          {/* ─── Three Option Cards ─── */}
          <div className="grid grid-cols-1 sm:grid-cols-[1.2fr_0.8fr] gap-4 w-full max-w-3xl">

            {/* ── Card 1: Continue on Web ── GREEN / primary — DOMINANT */}
            <Link
              href="/dashboard"
              className="group relative flex flex-col items-center justify-center text-center p-6 sm:p-8 bg-gradient-to-b from-[#00e5a010] to-[#0a0f1a] border-2 border-breathe rounded-sm transition-all duration-300 hover:scale-[1.02] hover:border-[#00e5a0] hover:shadow-[0_0_50px_#00e5a025,0_0_120px_#00e5a010] pulse-glow scale-in-delay-1 sm:row-span-2"
            >
              {/* Top accent bar */}
              <div className="absolute top-0 left-0 right-0 h-[2px] bg-gradient-to-r from-transparent via-[#00e5a0] to-transparent opacity-70 group-hover:opacity-100 transition-opacity duration-300" />

              {/* Spinning globe */}
              <div className="relative w-14 h-14 flex items-center justify-center mb-4">
                <div className="absolute inset-0 border border-[#00e5a0]/20 rounded-full spin-slow" />
                <svg
                  width="28"
                  height="28"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#00e5a0"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="float"
                >
                  <circle cx="12" cy="12" r="10" />
                  <path d="M2 12h20" />
                  <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10A15.3 15.3 0 0 1 12 2z" />
                </svg>
              </div>

              <h2 className="font-display text-2xl font-bold text-[#00e5a0] glow-text-strong mb-1.5">
                Continue on Web
              </h2>
              <p className="font-mono text-xs text-white leading-relaxed mb-4">
                For beginners &amp; terminal haters
              </p>

              {/* Solid green CTA button */}
              <div className="px-6 py-2.5 bg-[#00e5a0] rounded-sm font-mono text-sm font-semibold text-[#030712] flex items-center gap-2 group-hover:bg-[#00cc8e] group-hover:shadow-[0_0_25px_#00e5a030] transition-all duration-300">
                <span>Get Started</span>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="group-hover:translate-x-0.5 transition-transform duration-300"
                >
                  <line x1="5" y1="12" x2="19" y2="12" />
                  <polyline points="12 5 19 12 12 19" />
                </svg>
              </div>
            </Link>

            {/* ── Card 2: Switch to CLI ── AMBER / raw terminal */}
            <Link
              href="/cli"
              className="group relative flex flex-col items-center justify-center text-center p-5 sm:p-6 bg-[#0a0f1a] border border-[#1e293b] rounded-sm transition-all duration-300 hover:scale-[1.02] hover:border-[#f59e0b]/40 hover:bg-[#0f1520] hover:shadow-[0_0_40px_#f59e0b15,0_0_80px_#f59e0b08] scale-in-delay-2"
            >
              {/* Terminal prompt icon */}
              <div className="relative w-12 h-12 flex items-center justify-center mb-3 border border-[#1e293b] rounded-sm group-hover:border-[#f59e0b]/30 transition-all duration-300">
                <svg
                  width="22"
                  height="22"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#f59e0b"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="group-hover:drop-shadow-[0_0_10px_#f59e0b40] transition-all duration-300"
                >
                  <polyline points="4 17 10 11 4 5" />
                  <line x1="12" y1="19" x2="20" y2="19" />
                </svg>
              </div>

              <h2 className="font-display text-lg font-bold text-[#f59e0b] mb-1 group-hover:drop-shadow-[0_0_12px_#f59e0b30] transition-all duration-300">
                Switch to CLI
              </h2>
              <p className="font-mono text-[11px] text-white leading-relaxed">
                The full Myro experience
              </p>

              {/* Terminal typing prompt */}
              <div className="mt-3 px-3 py-1.5 bg-[#0a0a0f] border border-[#1e293b] rounded-sm font-mono text-[11px] text-[#f59e0b]/80 group-hover:border-[#f59e0b]/30 transition-all duration-300 flex items-center gap-1.5 whitespace-nowrap">
                <span className="text-[#64748b] flex-shrink-0">$</span>
                <span className="terminal-line">cargo install myro</span>
              </div>
            </Link>

            {/* ── Card 3: How it Works ── VIOLET / knowledge */}
            <button
              onClick={() =>
                document
                  .getElementById("how-it-works")
                  ?.scrollIntoView({ behavior: "smooth" })
              }
              className="group relative flex flex-col items-center justify-center text-center p-5 sm:p-6 bg-[#0a0f1a] border border-[#1e293b] rounded-sm transition-all duration-300 hover:scale-[1.02] hover:border-[#7c3aed]/40 hover:bg-[#0d0f1d] hover:shadow-[0_0_40px_#7c3aed15,0_0_80px_#7c3aed08] cursor-pointer scale-in-delay-3 pulse-ring"
            >
              {/* Lightbulb icon */}
              <div className="relative w-12 h-12 flex items-center justify-center mb-3 border border-[#1e293b] rounded-sm group-hover:border-[#7c3aed]/30 transition-all duration-300">
                <svg
                  width="22"
                  height="22"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#7c3aed"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="group-hover:drop-shadow-[0_0_10px_#7c3aed40] transition-all duration-300"
                >
                  <path d="M9 18h6" />
                  <path d="M10 22h4" />
                  <path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14" />
                </svg>
              </div>

              <h2 className="font-display text-lg font-bold text-[#7c3aed] mb-1 group-hover:drop-shadow-[0_0_12px_#7c3aed30] transition-all duration-300">
                How it Works
              </h2>
              <p className="font-mono text-[11px] text-white leading-relaxed">
                See the training method
              </p>

              {/* Animated down arrow */}
              <div className="mt-3 flex flex-col items-center gap-0">
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#7c3aed"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="animate-bounce opacity-60"
                >
                  <polyline points="6 9 12 15 18 9" />
                </svg>
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#7c3aed"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="animate-bounce opacity-30 -mt-2"
                  style={{ animationDelay: "0.15s" }}
                >
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </div>
            </button>
          </div>
        </div>
      </section>

      {/* ═══════════════ HOW IT WORKS ═══════════════ */}
      <section
        id="how-it-works"
        ref={howItWorks.ref}
        className="relative px-4 pt-24 pb-12 sm:pt-32 sm:pb-16"
      >
        <div className="max-w-4xl mx-auto">
          {/* Section header */}
          <div
            className={`text-center mb-16 transition-all duration-700 ${
              howItWorks.inView
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-4"
            }`}
          >
            <span className="font-mono text-xs tracking-[0.3em] uppercase text-[#94a3b8]">
              How it works
            </span>
            <h2 className="font-display text-3xl sm:text-4xl font-bold text-white mt-3">
              Three steps to{" "}
              <span className="text-[#00e5a0] glow-text">mastery</span>
            </h2>
          </div>

          {/* Steps */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-0 relative">
            {/* Connecting line (desktop only) */}
            <div className="hidden md:block absolute top-10 left-[16.67%] right-[16.67%]">
              {/* Base dashed line */}
              <div className="border-t border-dashed border-[#1e293b]" />
              {/* Animated glow line that extends as steps activate */}
              <div
                className="absolute top-[-1px] left-0 h-[2px] bg-gradient-to-r from-[#00e5a0] via-[#00e5a0] to-[#00e5a060] transition-all duration-700 ease-out"
                style={{
                  width:
                    activeStep >= 2
                      ? "100%"
                      : activeStep >= 1
                        ? "50%"
                        : activeStep >= 0
                          ? "0%"
                          : "0%",
                  opacity: activeStep >= 1 ? 1 : 0,
                  boxShadow:
                    activeStep >= 1
                      ? "0 0 8px #00e5a040, 0 0 20px #00e5a020"
                      : "none",
                }}
              />
            </div>

            {STEPS.map((step, i) => {
              const isActive = activeStep >= i;
              const isLightingUp = activeStep === i;

              return (
                <div
                  key={step.number}
                  className={`relative flex flex-col items-center text-center px-6 py-6 transition-all duration-700 ${
                    howItWorks.inView
                      ? "opacity-100 translate-y-0"
                      : "opacity-0 translate-y-6"
                  }`}
                  style={{
                    transitionDelay: howItWorks.inView
                      ? `${200 + i * 150}ms`
                      : "0ms",
                  }}
                >
                  {/* Number */}
                  <div
                    className="relative z-10 w-20 h-20 flex items-center justify-center bg-[#030712] border rounded-sm transition-all duration-700"
                    style={{
                      borderColor: isActive
                        ? "rgba(0, 229, 160, 0.5)"
                        : "#1e293b",
                      boxShadow: isLightingUp
                        ? "0 0 30px #00e5a030, 0 0 60px #00e5a015"
                        : isActive
                          ? "0 0 20px #00e5a015"
                          : "none",
                    }}
                  >
                    <span
                      className="font-display text-2xl font-bold transition-all duration-700"
                      style={{
                        color: isActive ? "#00e5a0" : "#1e293b",
                        textShadow: isActive
                          ? "0 0 10px #00e5a060, 0 0 30px #00e5a030, 0 0 60px #00e5a015"
                          : "none",
                      }}
                    >
                      {step.number}
                    </span>
                  </div>

                  {/* Title */}
                  <h3
                    className="font-display text-lg font-semibold mt-5 transition-all duration-700"
                    style={{
                      color: isActive ? "#ffffff" : "#334155",
                    }}
                  >
                    {step.title}
                  </h3>

                  {/* Description */}
                  <p
                    className="font-mono text-sm mt-3 leading-relaxed max-w-xs transition-all duration-700"
                    style={{
                      color: isActive ? "#cbd5e1" : "#1e293b",
                    }}
                  >
                    {step.description}
                  </p>
                </div>
              );
            })}
          </div>
        </div>
      </section>

      {/* ═══════════════ WHY MYRO ═══════════════ */}
      <section ref={whyMyro.ref} className="relative px-4 py-12 sm:py-16">
        <div className="max-w-4xl mx-auto">
          <div
            className={`text-center mb-12 transition-all duration-700 ${
              whyMyro.inView
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-4"
            }`}
          >
            <span className="font-mono text-xs tracking-[0.3em] uppercase text-[#94a3b8]">
              Why Myro
            </span>
            <h2 className="font-display text-2xl sm:text-3xl font-bold text-white mt-3">
              Not another{" "}
              <span className="text-[#00e5a0] glow-text">grind tool</span>
            </h2>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
            {[
              {
                comment: "// the problem",
                commentColor: "#00e5a0",
                text: (
                  <>
                    Most people memorize solutions and pattern-match. They solve 500
                    problems and still freeze on contest day because they never
                    learned <span className="text-white font-semibold">how to think</span>.
                  </>
                ),
              },
              {
                comment: "// the fix",
                commentColor: "#00e5a0",
                text: (
                  <>
                    Myro coaches you through the{" "}
                    <span className="text-white font-semibold">observations</span> behind
                    each solution — the key insights that unlock the approach. You
                    discover them yourself, guided by AI.
                  </>
                ),
              },
              {
                comment: "// adaptive hints",
                commentColor: "#7c3aed",
                text: (
                  <>
                    Stuck? The coach gives you progressively specific hints without
                    spoiling the answer. You build{" "}
                    <span className="text-white font-semibold">real intuition</span>,
                    not a false sense of progress.
                  </>
                ),
              },
              {
                comment: "// web + cli",
                commentColor: "#f59e0b",
                text: (
                  <>
                    Train on the web or install the CLI for a full terminal-native
                    experience. Same problems, same progress —{" "}
                    <span className="text-white font-semibold">your choice</span>.
                  </>
                ),
              },
            ].map((card, i) => {
              const isActive = activeCard >= i;

              return (
                <div
                  key={card.comment}
                  className="bg-[#0a0f1a] border rounded-sm p-6 transition-all duration-700"
                  style={{
                    opacity: isActive ? 1 : 0,
                    transform: isActive ? "translateY(0)" : "translateY(16px)",
                    borderColor: isActive ? "#1e293b" : "#0a0f1a",
                  }}
                >
                  <div
                    className="font-mono text-xs mb-2 transition-all duration-500"
                    style={{
                      color: isActive ? card.commentColor : "#1e293b",
                      transitionDelay: isActive ? "200ms" : "0ms",
                    }}
                  >
                    {card.comment}
                  </div>
                  <p
                    className="font-mono text-sm leading-relaxed transition-all duration-500"
                    style={{
                      color: isActive ? "#cbd5e1" : "#0a0f1a",
                      transitionDelay: isActive ? "350ms" : "0ms",
                    }}
                  >
                    {card.text}
                  </p>
                </div>
              );
            })}
          </div>

          {/* Back to top */}
          <div
            className={`flex justify-center mt-16 transition-all duration-700 ${
              activeCard >= 3
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-4"
            }`}
          >
            <button
              onClick={() =>
                window.scrollTo({ top: 0, behavior: "smooth" })
              }
              className="group flex items-center gap-2 font-mono text-xs text-[#94a3b8] hover:text-[#00e5a0] transition-colors cursor-pointer"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="group-hover:-translate-y-0.5 transition-transform duration-200"
              >
                <polyline points="18 15 12 9 6 15" />
              </svg>
              back to top
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
