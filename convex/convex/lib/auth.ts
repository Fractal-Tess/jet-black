import type { GenericCtx } from "@convex-dev/better-auth";
import { ConvexError } from "convex/values";

import type { DataModel } from "../_generated/dataModel";
import { authComponent } from "../auth";

export async function requireAuthUser(ctx: GenericCtx<DataModel>) {
  const user = await authComponent.getAuthUser(ctx);

  if (!user) {
    throw new ConvexError("Unauthenticated");
  }

  return user;
}
