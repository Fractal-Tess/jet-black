import { defineSchema } from "convex/server";

import { intakeTables } from "./schema/intake";
import { issueTables } from "./schema/issues";
import { messageTables } from "./schema/messages";
import { moduleTables } from "./schema/modules";
import { pageTables } from "./schema/pages";
import { sprintTables } from "./schema/sprints";
import { workspaceTables } from "./schema/workspaces";

export default defineSchema(
  {
    ...intakeTables,
    ...issueTables,
    ...messageTables,
    ...moduleTables,
    ...pageTables,
    ...sprintTables,
    ...workspaceTables,
  },
  { schemaValidation: true }
);
