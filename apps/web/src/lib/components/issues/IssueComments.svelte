<script lang="ts">
import type { Issue, IssueComment } from "./types";

let {
  issue,
  comments,
  onAddComment,
}: {
  issue: Issue;
  comments: IssueComment[];
  onAddComment: (body: string) => Promise<void>;
} = $props();

let commentBody = $state("");
let adding = $state(false);

async function addComment() {
  const body = commentBody.trim();

  if (!body) {
    return;
  }

  adding = true;
  try {
    await onAddComment(body);
    commentBody = "";
  } finally {
    adding = false;
  }
}
</script>

<div class="px-5 py-4">
  <h3 class="text-sm font-medium text-zinc-300">Activity</h3>

  <div class="mt-4 space-y-4">
    {#each comments as comment (comment._id)}
      <div class="flex gap-3">
        <span class="mt-0.5 grid size-6 shrink-0 place-items-center rounded-full bg-zinc-700 text-[10px] font-medium text-zinc-300">
          {comment.authorUserId.slice(0, 2).toUpperCase()}
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-sm text-zinc-300">{comment.body}</p>
          <p class="mt-1 text-[11px] text-zinc-600">
            {comment.authorUserId.slice(0, 8)}
          </p>
        </div>
      </div>
    {:else}
      <p class="text-sm text-zinc-600">
        No activity yet.
      </p>
    {/each}
  </div>

  <div class="mt-4">
    <label class="block">
      <span class="sr-only">New comment</span>
      <textarea
        bind:value={commentBody}
        class="min-h-[72px] w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
        placeholder="Leave a comment..."
      ></textarea>
    </label>
    <button
      class="mt-2 h-8 rounded-md bg-amber-400 px-3 text-xs font-medium text-black transition hover:bg-amber-300 disabled:opacity-50"
      disabled={adding || !commentBody.trim()}
      onclick={addComment}
      type="button"
    >
      {adding ? "Posting\u2026" : "Comment"}
    </button>
  </div>
</div>
