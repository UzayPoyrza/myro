import { NextRequest, NextResponse } from "next/server";
import { prisma } from "@/lib/db";

const DEMO_USER_EMAIL = "demo@myro.dev";

/**
 * POST /api/sessions
 * Create a new training session for a problem.
 * Body: { problemId: string, routeId?: string }
 */
export async function POST(req: NextRequest) {
  const body = await req.json();
  const { problemId, routeId } = body;

  if (!problemId) {
    return NextResponse.json(
      { error: "problemId is required" },
      { status: 400 }
    );
  }

  // Get or create demo user
  const user = await prisma.user.upsert({
    where: { email: DEMO_USER_EMAIL },
    update: {},
    create: { email: DEMO_USER_EMAIL, name: "Demo User" },
  });

  // If no routeId, use the first route of the problem
  let selectedRouteId = routeId;
  if (!selectedRouteId) {
    const firstRoute = await prisma.route.findFirst({
      where: { problemId },
      orderBy: { order: "asc" },
    });
    selectedRouteId = firstRoute?.id ?? null;
  }

  const session = await prisma.trainingSession.create({
    data: {
      userId: user.id,
      problemId,
      routeId: selectedRouteId,
      status: "active",
    },
    include: {
      problem: true,
      route: {
        include: { observations: { orderBy: { order: "asc" } } },
      },
      messages: { orderBy: { createdAt: "asc" } },
      unlocked: true,
    },
  });

  return NextResponse.json(session);
}

/**
 * GET /api/sessions?id=xxx
 * Get a session by ID with all related data.
 */
export async function GET(req: NextRequest) {
  const sessionId = req.nextUrl.searchParams.get("id");

  if (!sessionId) {
    return NextResponse.json(
      { error: "id query parameter required" },
      { status: 400 }
    );
  }

  const session = await prisma.trainingSession.findUnique({
    where: { id: sessionId },
    include: {
      problem: true,
      route: {
        include: { observations: { orderBy: { order: "asc" } } },
      },
      messages: { orderBy: { createdAt: "asc" } },
      unlocked: true,
    },
  });

  if (!session) {
    return NextResponse.json({ error: "Session not found" }, { status: 404 });
  }

  return NextResponse.json(session);
}
