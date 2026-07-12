import { defineSchema } from "convex/server";

import { issueTables } from "./schema/issues";
import { messageTables } from "./schema/messages";
import { workspaceTables } from "./schema/workspaces";

export default defineSchema(
  {
    ...issueTables,
    ...messageTables,
    ...workspaceTables,
  },
  { schemaValidation: true }
);
