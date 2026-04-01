import { redirect } from "next/navigation";
import { prisma } from "@/lib/db";

const DEMO_USER_EMAIL = "demo@myro.dev";

/**
 * Starting a training session: creates a new session and redirects to it.
 */
export default async function StartSessionPage({
  params,
}: {
  params: Promise<{ problemId: string }>;
}) {
  const { problemId } = await params;

  // Verify problem exists
  const problem = await prisma.problem.findUnique({
    where: { id: problemId },
    include: {
      routes: { orderBy: { order: "asc" } },
    },
  });

  if (!problem) {
    redirect("/problems");
  }

  // Get or create demo user
  const user = await prisma.user.upsert({
    where: { email: DEMO_USER_EMAIL },
    update: {},
    create: { email: DEMO_USER_EMAIL, name: "Demo User" },
  });

  // Check for existing active session
  const existing = await prisma.trainingSession.findFirst({
    where: {
      userId: user.id,
      problemId,
      status: { in: ["active", "implementing"] },
    },
  });

  if (existing) {
    redirect(`/train/${existing.id}`);
  }

  // Create new session with first route
  const firstRoute = problem.routes[0];
  const session = await prisma.trainingSession.create({
    data: {
      userId: user.id,
      problemId,
      routeId: firstRoute?.id ?? null,
      status: "active",
    },
  });

  redirect(`/train/${session.id}`);
}
