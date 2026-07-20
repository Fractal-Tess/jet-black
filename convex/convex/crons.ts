import { cronJobs } from "convex/server";

import { internal } from "./_generated/api";

const crons = cronJobs();

crons.cron(
  "project automations",
  "0 3 * * *",
  internal.mutations.automations.runAll,
  {}
);

export default crons;
