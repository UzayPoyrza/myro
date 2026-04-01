import { NextRequest, NextResponse } from "next/server";
import { prisma } from "@/lib/db";
import { getProvider } from "@/lib/llm/provider";
import { buildSystemPrompt, buildChatMessages } from "@/lib/coach/prompt";
import {
  parseCoachResponse,
  FALLBACK_RESPONSE,
  type CoachResponse,
} from "@/lib/coach/schema";

/**
 * POST /api/chat
 * Send a user message and get a coach response.
 * Body: { sessionId: string, message: string }
 */
export async function POST(req: NextRequest) {
  const body = await req.json();
  const { sessionId, message } = body;

  if (!sessionId || !message) {
    return NextResponse.json(
      { error: "sessionId and message are required" },
      { status: 400 }
    );
  }

  // Load session with all context
  const session = await prisma.trainingSession.findUnique({
    where: { id: sessionId },
    include: {
      problem: true,
      route: {
        include: { observations: { orderBy: { order: "asc" } } },
      },
      messages: { orderBy: { createdAt: "asc" }, take: 20 },
      unlocked: true,
    },
  });

  if (!session || !session.route) {
    return NextResponse.json(
      { error: "Session or route not found" },
      { status: 404 }
    );
  }

  // Save user message
  await prisma.chatMessage.create({
    data: {
      sessionId,
      role: "user",
      content: message,
    },
  });

  // Build LLM context
  const unlockedIds = new Set(session.unlocked.map((u) => u.observationId));
  const hintLevels = new Map(
    session.unlocked.map((u) => [u.observationId, u.hintLevel])
  );

  const systemPrompt = buildSystemPrompt({
    problem: session.problem,
    route: session.route,
    observations: session.route.observations,
    unlockedIds,
    hintLevels,
    recentMessages: session.messages,
  });

  const chatMessages = buildChatMessages(systemPrompt, [
    ...session.messages,
    { id: "", sessionId, role: "user", content: message, metadata: null, createdAt: new Date() },
  ]);

  // Call LLM with retry
  let coachResponse: CoachResponse;
  try {
    const provider = getProvider();
    const raw = await provider.chat(systemPrompt, chatMessages, {
      temperature: 0.7,
      maxTokens: 1024,
    });

    const parsed = parseCoachResponse(raw);
    if (parsed) {
      coachResponse = parsed;
    } else {
      // Retry with stricter prompt
      const retryRaw = await provider.chat(
        systemPrompt +
          "\n\nIMPORTANT: Your previous response was invalid JSON. Respond with ONLY valid JSON, no markdown fences, no extra text.",
        chatMessages,
        { temperature: 0.3, maxTokens: 1024 }
      );
      const retryParsed = parseCoachResponse(retryRaw);
      coachResponse = retryParsed ?? FALLBACK_RESPONSE;
    }
  } catch (error) {
    console.error("LLM call failed:", error);
    coachResponse = FALLBACK_RESPONSE;
  }

  // Save coach message
  await prisma.chatMessage.create({
    data: {
      sessionId,
      role: "coach",
      content: coachResponse.coach_message,
      metadata: JSON.stringify(coachResponse),
    },
  });

  // If observation found with high confidence, unlock it
  let newUnlock = null;
  if (
    coachResponse.state === "found" &&
    coachResponse.confidence > 0.8 &&
    coachResponse.matched_observation_id &&
    !unlockedIds.has(coachResponse.matched_observation_id)
  ) {
    // Verify the observation belongs to this route
    const validObs = session.route.observations.find(
      (o) => o.id === coachResponse.matched_observation_id
    );
    if (validObs) {
      newUnlock = await prisma.unlockedObservation.create({
        data: {
          sessionId,
          observationId: coachResponse.matched_observation_id,
          hintLevel: 0,
        },
      });
    }
  }

  // Check if all observations are now unlocked
  const totalObs = session.route.observations.length;
  const unlockedCount = unlockedIds.size + (newUnlock ? 1 : 0);
  let sessionStatus = session.status;

  if (unlockedCount >= totalObs && sessionStatus === "active") {
    sessionStatus = "implementing";
    await prisma.trainingSession.update({
      where: { id: sessionId },
      data: { status: "implementing" },
    });
  }

  return NextResponse.json({
    coachResponse,
    newUnlock,
    unlockedCount,
    totalObservations: totalObs,
    sessionStatus,
  });
}
