import { defineSchema } from "convex/server";

import { intakeTables } from "./schema/intake";
import { issueTables } from "./schema/issues";
import { messageTables } from "./schema/messages";
import { moduleTables } from "./schema/modules";
import { sprintTables } from "./schema/sprints";
import { workspaceTables } from "./schema/workspaces";

export default defineSchema(
  {
    ...intakeTables,
    ...issueTables,
    ...messageTables,
    ...moduleTables,
    ...sprintTables,
    ...workspaceTables,
  },
  { schemaValidation: true }
);
