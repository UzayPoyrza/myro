import { NextRequest, NextResponse } from "next/server";
import { prisma } from "@/lib/db";

/**
 * POST /api/hints
 * Advance the hint ladder for the nearest relevant observation.
 * Body: { sessionId: string, observationId?: string }
 *
 * If observationId is not provided, advances the hint for the first
 * locked observation in the route.
 */
export async function POST(req: NextRequest) {
  const body = await req.json();
  const { sessionId, observationId } = body;

  if (!sessionId) {
    return NextResponse.json(
      { error: "sessionId is required" },
      { status: 400 }
    );
  }

  const session = await prisma.trainingSession.findUnique({
    where: { id: sessionId },
    include: {
      route: {
        include: { observations: { orderBy: { order: "asc" } } },
      },
      unlocked: true,
    },
  });

  if (!session || !session.route) {
    return NextResponse.json(
      { error: "Session or route not found" },
      { status: 404 }
    );
  }

  const unlockedIds = new Set(session.unlocked.map((u) => u.observationId));
  const unlockedMap = new Map(
    session.unlocked.map((u) => [u.observationId, u])
  );

  // Find target observation
  let targetObsId = observationId;
  if (!targetObsId) {
    // Find first locked observation
    const firstLocked = session.route.observations.find(
      (o) => !unlockedIds.has(o.id)
    );
    if (!firstLocked) {
      return NextResponse.json(
        { error: "All observations already unlocked" },
        { status: 400 }
      );
    }
    targetObsId = firstLocked.id;
  }

  const targetObs = session.route.observations.find(
    (o) => o.id === targetObsId
  );
  if (!targetObs) {
    return NextResponse.json(
      { error: "Observation not found in this route" },
      { status: 404 }
    );
  }

  const hints = JSON.parse(targetObs.hints) as string[];
  const existing = unlockedMap.get(targetObsId);

  let currentHintLevel = existing?.hintLevel ?? 0;
  const nextHintLevel = Math.min(currentHintLevel + 1, 3);

  if (nextHintLevel > 3) {
    return NextResponse.json(
      { error: "Maximum hint level reached" },
      { status: 400 }
    );
  }

  // Upsert the unlock record with the new hint level
  const unlock = await prisma.unlockedObservation.upsert({
    where: {
      sessionId_observationId: {
        sessionId,
        observationId: targetObsId,
      },
    },
    create: {
      sessionId,
      observationId: targetObsId,
      hintLevel: nextHintLevel,
    },
    update: {
      hintLevel: nextHintLevel,
    },
  });

  // Get the hint text for this level (0-indexed, so level 1 = hints[0])
  const hintText = hints[nextHintLevel - 1] ?? "Think about this further...";

  // Save hint as a coach message
  await prisma.chatMessage.create({
    data: {
      sessionId,
      role: "coach",
      content: `💡 Hint (level ${nextHintLevel}/3): ${hintText}`,
      metadata: JSON.stringify({
        type: "hint",
        observationId: targetObsId,
        hintLevel: nextHintLevel,
      }),
    },
  });

  return NextResponse.json({
    observationId: targetObsId,
    hintLevel: nextHintLevel,
    hintText,
    unlock,
  });
}
