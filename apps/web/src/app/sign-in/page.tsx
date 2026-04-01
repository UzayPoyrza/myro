"use client";

import Link from "next/link";
import { useState } from "react";

type AuthMode = "signin" | "signup";

function GitHubIcon() {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844a9.59 9.59 0 0 1 2.504.337c1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.02 10.02 0 0 0 22 12.017C22 6.484 17.522 2 12 2z"
        fill="currentColor"
      />
    </svg>
  );
}

function GoogleIcon() {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 0 1-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z"
        fill="#4285F4"
      />
      <path
        d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
        fill="#34A853"
      />
      <path
        d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
        fill="#FBBC05"
      />
      <path
        d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
        fill="#EA4335"
      />
    </svg>
  );
}

export default function SignInPage() {
  const [mode, setMode] = useState<AuthMode>("signin");

  return (
    <div className="relative min-h-dvh flex items-center justify-center px-4 py-12">
      {/* Card */}
      <div className="w-full max-w-[420px] bg-[#0a0f1a] border border-[#1e293b] rounded-sm shadow-[0_0_80px_#00e5a008,0_0_160px_#00e5a004] fade-in">
        {/* Logo */}
        <div className="flex flex-col items-center pt-10 pb-2 fade-in">
          <Link href="/" className="flex items-center gap-1">
            <span className="font-display text-xl font-bold tracking-wide text-[#00e5a0] glow-text-strong">
              MYRO
            </span>
            <span className="font-display text-xl font-bold tracking-wide text-[#e2e8f0]">
              Web
            </span>
          </Link>
          <p className="font-mono text-xs text-[#94a3b8] mt-3">
            {mode === "signin"
              ? "Welcome back, commander."
              : "Join the ranks, commander."}
          </p>
        </div>

        {/* Card body */}
        <div className="px-8 pb-10 pt-6">
          {/* Tab toggle */}
          <div className="flex mb-8 border-b border-[#1e293b] fade-in-delay-1">
            <button
              onClick={() => setMode("signin")}
              className={`flex-1 pb-3 font-mono text-xs tracking-wide transition-all duration-200 cursor-pointer ${
                mode === "signin"
                  ? "text-[#00e5a0] border-b-2 border-[#00e5a0]"
                  : "text-[#64748b] border-b-2 border-transparent hover:text-[#94a3b8]"
              }`}
            >
              Sign in
            </button>
            <button
              onClick={() => setMode("signup")}
              className={`flex-1 pb-3 font-mono text-xs tracking-wide transition-all duration-200 cursor-pointer ${
                mode === "signup"
                  ? "text-[#00e5a0] border-b-2 border-[#00e5a0]"
                  : "text-[#64748b] border-b-2 border-transparent hover:text-[#94a3b8]"
              }`}
            >
              Sign up
            </button>
          </div>

          {/* OAuth buttons */}
          <div className="flex flex-col gap-3 fade-in-delay-2">
            <button
              type="button"
              onClick={(e) => e.preventDefault()}
              className="group flex items-center justify-center gap-3 w-full px-4 py-3 bg-[#0a0f1a] border border-[#1e293b] rounded-sm font-mono text-sm text-[#e2e8f0] transition-all duration-200 hover:border-[#334155] hover:bg-[#0f1520] hover:-translate-y-px cursor-pointer"
            >
              <GitHubIcon />
              <span>Continue with GitHub</span>
            </button>

            <button
              type="button"
              onClick={(e) => e.preventDefault()}
              className="group flex items-center justify-center gap-3 w-full px-4 py-3 bg-[#0a0f1a] border border-[#1e293b] rounded-sm font-mono text-sm text-[#e2e8f0] transition-all duration-200 hover:border-[#334155] hover:bg-[#0f1520] hover:-translate-y-px cursor-pointer"
            >
              <GoogleIcon />
              <span>Continue with Google</span>
            </button>
          </div>

          {/* Divider */}
          <div className="flex items-center gap-4 my-7 fade-in-delay-3">
            <div className="flex-1 h-px bg-[#1e293b]" />
            <span className="font-mono text-xs text-[#334155]">or</span>
            <div className="flex-1 h-px bg-[#1e293b]" />
          </div>

          {/* Form */}
          <form
            onSubmit={(e) => e.preventDefault()}
            className="flex flex-col gap-5 fade-in-delay-3"
          >
            {/* Email field */}
            <div className="flex flex-col gap-1.5">
              <label
                htmlFor="email"
                className="font-mono text-xs text-[#64748b]"
              >
                Email
              </label>
              <input
                id="email"
                type="email"
                autoComplete="email"
                placeholder="you@example.com"
                className="w-full px-3.5 py-2.5 bg-[#060b14] border border-[#1e293b] rounded-sm font-mono text-sm text-[#e2e8f0] placeholder:text-[#334155] outline-none transition-all duration-200 focus:border-[#00e5a0]/50 focus:shadow-[0_0_0_1px_#00e5a020,0_0_12px_#00e5a010]"
              />
            </div>

            {/* Password field */}
            <div className="flex flex-col gap-1.5">
              <div className="flex items-center justify-between">
                <label
                  htmlFor="password"
                  className="font-mono text-xs text-[#64748b]"
                >
                  Password
                </label>
                {mode === "signin" && (
                  <button
                    type="button"
                    className="font-mono text-xs text-[#64748b] hover:text-[#00e5a0] transition-colors duration-200 cursor-pointer"
                  >
                    Forgot?
                  </button>
                )}
              </div>
              <input
                id="password"
                type="password"
                autoComplete={
                  mode === "signin" ? "current-password" : "new-password"
                }
                placeholder="••••••••"
                className="w-full px-3.5 py-2.5 bg-[#060b14] border border-[#1e293b] rounded-sm font-mono text-sm text-[#e2e8f0] placeholder:text-[#334155] outline-none transition-all duration-200 focus:border-[#00e5a0]/50 focus:shadow-[0_0_0_1px_#00e5a020,0_0_12px_#00e5a010]"
              />
            </div>

            {/* Submit button */}
            <button
              type="submit"
              className="w-full mt-1 px-4 py-3 bg-[#00e5a0] rounded-sm font-mono text-sm font-semibold text-[#030712] transition-all duration-200 hover:bg-[#00cc8e] hover:shadow-[0_0_25px_#00e5a030] active:scale-[0.98] cursor-pointer"
            >
              {mode === "signin" ? "Sign in" : "Create account"}
            </button>
          </form>

          {/* Bottom link */}
          <p className="text-center font-mono text-xs text-[#64748b] mt-7 fade-in-delay-4">
            {mode === "signin" ? (
              <>
                Don&apos;t have an account?{" "}
                <Link
                  href="/sign-up"
                  onClick={(e) => {
                    e.preventDefault();
                    setMode("signup");
                  }}
                  className="text-[#00e5a0] hover:underline transition-colors duration-200"
                >
                  Sign up
                </Link>
              </>
            ) : (
              <>
                Already have an account?{" "}
                <button
                  type="button"
                  onClick={() => setMode("signin")}
                  className="text-[#00e5a0] hover:underline transition-colors duration-200 cursor-pointer"
                >
                  Sign in
                </button>
              </>
            )}
          </p>
        </div>
      </div>
    </div>
  );
}
