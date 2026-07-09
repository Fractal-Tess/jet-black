import { mutation } from "./_generated/server";
import { buildSeedMessage } from "./messages";

export const seed = mutation({
  args: {},
  handler: async (ctx) => {
    let messagesInserted = 0;

    const existingMessage = await ctx.db.query("messages").take(1);
    if (existingMessage.length === 0) {
      await ctx.db.insert("messages", {
        body: buildSeedMessage("web app"),
        source: "seed",
      });
      messagesInserted = 1;
    }

    return {
      messagesInserted,
      seeded: messagesInserted > 0,
    };
  },
});
