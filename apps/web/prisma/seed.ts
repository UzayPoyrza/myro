import { PrismaClient } from "../src/generated/prisma/client";
import { PrismaBetterSqlite3 } from "@prisma/adapter-better-sqlite3";
import { SEED_PROBLEMS } from "../src/lib/seed-data";

const adapter = new PrismaBetterSqlite3({
  url: process.env.DATABASE_URL ?? "file:./dev.db",
});
const prisma = new PrismaClient({ adapter });

async function main() {
  // Delete all records in correct order to respect foreign keys
  await prisma.chatMessage.deleteMany();
  await prisma.unlockedObservation.deleteMany();
  await prisma.observation.deleteMany();
  await prisma.route.deleteMany();
  await prisma.trainingSession.deleteMany();
  await prisma.problem.deleteMany();
  await prisma.user.deleteMany();

  // Create demo user
  const demoUser = await prisma.user.upsert({
    where: { email: "demo@myro.dev" },
    update: {},
    create: {
      email: "demo@myro.dev",
      name: "Demo User",
    },
  });

  console.log(`Created demo user: ${demoUser.email}`);

  // Iterate over seed problems and create Problem, Route, and Observation records
  for (const seedProblem of SEED_PROBLEMS) {
    const problem = await prisma.problem.upsert({
      where: { title: seedProblem.title },
      update: {
        difficulty: seedProblem.difficulty,
        topic: seedProblem.topic,
        description: seedProblem.description,
        inputSpec: seedProblem.inputSpec,
        outputSpec: seedProblem.outputSpec,
        examples: JSON.stringify(seedProblem.examples),
      },
      create: {
        title: seedProblem.title,
        difficulty: seedProblem.difficulty,
        topic: seedProblem.topic,
        description: seedProblem.description,
        inputSpec: seedProblem.inputSpec,
        outputSpec: seedProblem.outputSpec,
        examples: JSON.stringify(seedProblem.examples),
      },
    });

    console.log(`Upserted problem: ${problem.title}`);

    for (const seedRoute of seedProblem.routes) {
      const route = await prisma.route.upsert({
        where: {
          problemId_order: {
            problemId: problem.id,
            order: seedRoute.order,
          },
        },
        update: {
          name: seedRoute.name,
          description: seedRoute.description,
        },
        create: {
          name: seedRoute.name,
          description: seedRoute.description,
          order: seedRoute.order,
          problemId: problem.id,
        },
      });

      console.log(`  Upserted route: ${route.name}`);

      for (const seedObservation of seedRoute.observations) {
        const observation = await prisma.observation.upsert({
          where: {
            routeId_order: {
              routeId: route.id,
              order: seedObservation.order,
            },
          },
          update: {
            title: seedObservation.title,
            description: seedObservation.description,
            hints: JSON.stringify(seedObservation.hints),
          },
          create: {
            order: seedObservation.order,
            title: seedObservation.title,
            description: seedObservation.description,
            hints: JSON.stringify(seedObservation.hints),
            routeId: route.id,
          },
        });

        console.log(`    Upserted observation: ${observation.title}`);
      }
    }
  }

  console.log("Seed completed successfully.");
}

main()
  .catch((e) => {
    console.error(e);
    process.exit(1);
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
