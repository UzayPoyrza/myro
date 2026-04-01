import type { Observation, Problem, Route, ChatMessage, UnlockedObservation } from "../../generated/prisma/client";

interface PromptContext {
  problem: Problem;
  route: Route;
  observations: Observation[];
  unlockedIds: Set<string>;
  hintLevels: Map<string, number>; // observationId → hint level
  recentMessages: ChatMessage[];
}

export function buildSystemPrompt(ctx: PromptContext): string {
  const examples = JSON.parse(ctx.problem.examples) as {
    input: string;
    output: string;
  }[];

  const examplesText = examples
    .map(
      (ex, i) =>
        `  Example ${i + 1}:\n    Input: ${ex.input}\n    Output: ${ex.output}`
    )
    .join("\n");

  const observationsText = ctx.observations
    .map((obs) => {
      const status = ctx.unlockedIds.has(obs.id) ? "UNLOCKED" : "LOCKED";
      const hintLevel = ctx.hintLevels.get(obs.id) || 0;
      const hintNote =
        hintLevel > 0 ? ` (hint level ${hintLevel} given)` : "";
      return `- ${obs.id}: "${obs.title}" — ${obs.description} [${status}]${hintNote}`;
    })
    .join("\n");

  const unlockedTitles = ctx.observations
    .filter((obs) => ctx.unlockedIds.has(obs.id))
    .map((obs) => obs.title);

  const unlockedText =
    unlockedTitles.length > 0
      ? unlockedTitles.join(", ")
      : "None yet";

  return `You are Myro, an observation coach for competitive programming.

PROBLEM:
${ctx.problem.title} (difficulty: ${ctx.problem.difficulty})

${ctx.problem.description}

Input: ${ctx.problem.inputSpec}
Output: ${ctx.problem.outputSpec}

${examplesText}

ROUTE: ${ctx.route.name}
${ctx.route.description}

OBSERVATIONS (key insights the user must discover):
${observationsText}

UNLOCKED SO FAR: ${unlockedText}

RULES:
1. Detect which observation the user is approaching, has found, is moving away from, or is uncertain about.
2. Help the user make progress. Prefer questions and nudges, but give more direct help when they're stuck.
3. Confirm an observation as "found" ONLY when the user clearly demonstrates understanding of the core insight. Set confidence > 0.8 only when you are very sure.
4. Be encouraging and constructive. Name concepts and techniques when helpful.
5. Be honest about wrong directions — redirect clearly.
6. Keep coach_message under 500 characters.
7. Keep next_action under 200 characters.
8. Don't read observation titles verbatim to the user, but you can reference the underlying concept.
9. If the user seems lost, give increasingly direct help toward the most relevant locked observation.

You MUST respond with ONLY valid JSON (no markdown fences, no extra text) matching this exact schema:
{
  "state": "approaching" | "found" | "moving_away" | "uncertain",
  "confidence": <number 0.0-1.0>,
  "matched_observation_id": "<observation_id>" | null,
  "coach_message": "<short coaching message>",
  "next_action": "<one concrete action>",
  "why_signals": ["<signal1>", "<signal2>"]
}`;
}

export function buildChatMessages(
  systemPrompt: string,
  recentMessages: ChatMessage[]
): { role: "user" | "assistant"; content: string }[] {
  const messages: { role: "user" | "assistant"; content: string }[] = [];

  for (const msg of recentMessages) {
    if (msg.role === "user") {
      messages.push({ role: "user", content: msg.content });
    } else if (msg.role === "coach" && msg.metadata) {
      // For coach messages, send the original JSON so the LLM has context
      messages.push({ role: "assistant", content: msg.metadata });
    } else if (msg.role === "coach") {
      messages.push({ role: "assistant", content: msg.content });
    }
  }

  return messages;
}
