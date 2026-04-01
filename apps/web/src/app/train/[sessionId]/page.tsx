"use client";

import { useEffect, useState, useCallback } from "react";
import { ProblemStatement } from "@/components/problem-statement";
import { Chat } from "@/components/chat";
import { InsightMap } from "@/components/insight-map";
import { CodeEditor } from "@/components/code-editor";

interface Observation {
  id: string;
  order: number;
  title: string;
  description: string;
  hints: string;
}

interface Route {
  id: string;
  name: string;
  description: string;
  observations: Observation[];
}

interface Problem {
  id: string;
  title: string;
  difficulty: number;
  topic: string;
  description: string;
  inputSpec: string;
  outputSpec: string;
  examples: string;
}

interface ChatMsg {
  id: string;
  role: "user" | "coach";
  content: string;
  createdAt: string;
}

interface UnlockedObs {
  id: string;
  observationId: string;
  hintLevel: number;
}

interface Session {
  id: string;
  status: string;
  scratchpad: string;
  code: string;
  problem: Problem;
  route: Route | null;
  messages: ChatMsg[];
  unlocked: UnlockedObs[];
}

export default function TrainingSessionPage({
  params,
}: {
  params: Promise<{ sessionId: string }>;
}) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [loading, setLoading] = useState(true);
  const [chatLoading, setChatLoading] = useState(false);
  const [statementCollapsed, setStatementCollapsed] = useState(false);
  const [code, setCode] = useState("");

  // Unwrap params
  useEffect(() => {
    params.then((p) => setSessionId(p.sessionId));
  }, [params]);

  // Fetch session data
  const fetchSession = useCallback(async () => {
    if (!sessionId) return;
    try {
      const res = await fetch(`/api/sessions?id=${sessionId}`);
      if (res.ok) {
        const data = await res.json();
        setSession(data);
        setCode(data.code || "");
      }
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  useEffect(() => {
    fetchSession();
  }, [fetchSession]);

  const handleSendMessage = async (message: string) => {
    if (!session) return;

    // Optimistically add user message
    const optimisticMsg: ChatMsg = {
      id: `temp-${Date.now()}`,
      role: "user",
      content: message,
      createdAt: new Date().toISOString(),
    };
    setSession((prev) =>
      prev
        ? { ...prev, messages: [...prev.messages, optimisticMsg] }
        : prev
    );

    setChatLoading(true);
    try {
      const res = await fetch("/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sessionId: session.id, message }),
      });

      if (res.ok) {
        // Refresh full session to get updated messages and unlocks
        await fetchSession();
      }
    } finally {
      setChatLoading(false);
    }
  };

  const handleMoreHint = async () => {
    if (!session) return;
    setChatLoading(true);
    try {
      const res = await fetch("/api/hints", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sessionId: session.id }),
      });
      if (res.ok) {
        await fetchSession();
      }
    } finally {
      setChatLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-zinc-500">Loading session...</div>
      </div>
    );
  }

  if (!session || !session.route) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-zinc-500">Session not found.</div>
      </div>
    );
  }

  const examples = JSON.parse(session.problem.examples) as {
    input: string;
    output: string;
  }[];
  const unlockedIds = new Set(session.unlocked.map((u) => u.observationId));
  const hintLevelMap = new Map(
    session.unlocked.map((u) => [u.observationId, u.hintLevel])
  );
  const unlockedCount = session.unlocked.filter(
    (u) => u.hintLevel === 0
  ).length + session.unlocked.filter((u) => u.hintLevel > 0).length;

  const isImplementing = session.status === "implementing";

  return (
    <div className="min-h-screen flex flex-col">
      {/* Problem Statement */}
      <ProblemStatement
        title={session.problem.title}
        difficulty={session.problem.difficulty}
        topic={session.problem.topic}
        description={session.problem.description}
        inputSpec={session.problem.inputSpec}
        outputSpec={session.problem.outputSpec}
        examples={examples}
        collapsed={statementCollapsed}
        onToggle={() => setStatementCollapsed(!statementCollapsed)}
      />

      {/* Main content area */}
      <div className="flex-1 flex min-h-0">
        {/* Left panel: Chat or Code Editor */}
        <div className="flex-1 flex flex-col border-r border-zinc-800 min-w-0">
          {isImplementing ? (
            <div className="flex-1 flex flex-col p-4">
              <div className="mb-4">
                <h2 className="text-lg font-semibold text-teal-400 mb-2">
                  Implementation Mode
                </h2>
                <div className="space-y-2 text-sm">
                  <label className="flex items-center gap-2 text-zinc-300">
                    <input type="checkbox" className="accent-teal-500" />
                    State definition: what variables do you need?
                  </label>
                  <label className="flex items-center gap-2 text-zinc-300">
                    <input type="checkbox" className="accent-teal-500" />
                    Transition: how does state update per element?
                  </label>
                  <label className="flex items-center gap-2 text-zinc-300">
                    <input type="checkbox" className="accent-teal-500" />
                    Complexity: time and space analysis
                  </label>
                  <label className="flex items-center gap-2 text-zinc-300">
                    <input type="checkbox" className="accent-teal-500" />
                    Edge cases: what inputs could break your solution?
                  </label>
                </div>
              </div>
              <CodeEditor code={code} onChange={setCode} language="python" />
            </div>
          ) : (
            <Chat
              messages={session.messages}
              onSend={handleSendMessage}
              isLoading={chatLoading}
            />
          )}
        </div>

        {/* Right panel: Insight Map */}
        <div className="w-80 flex-shrink-0 p-4 overflow-y-auto">
          <InsightMap
            observations={session.route.observations.map((obs) => ({
              id: obs.id,
              order: obs.order,
              title: obs.title,
              isUnlocked: unlockedIds.has(obs.id),
              hintLevel: hintLevelMap.get(obs.id) ?? 0,
            }))}
            totalObservations={session.route.observations.length}
            unlockedCount={unlockedCount}
            routeName={session.route.name}
            onMoreHint={handleMoreHint}
          />
        </div>
      </div>
    </div>
  );
}
