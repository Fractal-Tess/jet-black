import { defineTable } from "convex/server";
import { v } from "convex/values";

export const messageTables = {
  messages: defineTable({
    body: v.string(),
    source: v.string(),
  }),
};
