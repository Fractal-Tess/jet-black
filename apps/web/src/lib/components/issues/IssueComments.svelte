<script lang="ts">
import DescriptionEditor from "./DescriptionEditor.svelte";
import { useDescriptionUploader } from "./description-upload";
import IssueActivityRow from "./IssueActivityRow.svelte";
import IssueCommentBlock from "./IssueCommentBlock.svelte";
import type { Issue, IssueActivity, IssueComment } from "./types";

let {
  activities = [],
  comments,
  issue,
  onAddComment,
  onDeleteComment,
  onUpdateComment,
}: {
  activities?: IssueActivity[];
  comments: IssueComment[];
  issue: Issue;
  onAddComment: (
    body: string,
    parentCommentId?: IssueComment["_id"]
  ) => Promise<void>;
  onDeleteComment: (commentId: IssueComment["_id"]) => Promise<void>;
  onUpdateComment: (
    commentId: IssueComment["_id"],
    body: string
  ) => Promise<void>;
} = $props();

type TimelineFilter = "comments" | "updates";

type TimelineEntry =
  | { kind: "comment"; ts: number; comment: IssueComment }
  | { kind: "activity"; ts: number; activity: IssueActivity };

const FILTERS: { label: string; value: TimelineFilter }[] = [
  { label: "Comments", value: "comments" },
  { label: "Updates", value: "updates" },
];

let filter = $state<TimelineFilter>("comments");
let adding = $state(false);
let commentDraft = $state("");
let composer = $state<DescriptionEditor | null>(null);

const uploadCommentFile = useDescriptionUploader(() => issue.workspaceId);

const repliesByParent = $derived.by(() => {
  const map = new Map<IssueComment["_id"], IssueComment[]>();

  for (const comment of comments) {
    if (!comment.parentCommentId) {
      continue;
    }
    const existing = map.get(comment.parentCommentId);

    if (existing) {
      existing.push(comment);
    } else {
      map.set(comment.parentCommentId, [comment]);
    }
  }

  return map;
});

const timeline = $derived.by(() => {
  const entries: TimelineEntry[] = [
    ...comments
      .filter((comment) => !comment.parentCommentId)
      .map((comment) => ({
        comment,
        kind: "comment" as const,
        ts: comment._creationTime,
      })),
    ...activities
      .filter((activity) => activity.message !== "commented")
      .map((activity) => ({
        activity,
        kind: "activity" as const,
        ts: activity._creationTime,
      })),
  ];

  const filtered = entries.filter((entry) =>
    filter === "comments" ? entry.kind === "comment" : entry.kind === "activity"
  );

  return filtered.toSorted((a, b) => a.ts - b.ts);
});

async function addComment() {
  const body = composer?.getMarkdown().trim() ?? "";

  if (!body) {
    return;
  }

  adding = true;
  try {
    await onAddComment(body);
    composer?.setMarkdown("");
    commentDraft = "";
  } finally {
    adding = false;
  }
}
</script>

<div class="px-5 py-4">
  <div class="flex items-center justify-between gap-2">
    <h3 class="text-sm font-medium text-foreground">Activity</h3>

    <div class="flex items-center rounded-lg bg-muted p-0.5">
      {#each FILTERS as option (option.value)}
        <button
          class="h-6 cursor-pointer rounded-md px-2.5 text-xs transition-colors {filter ===
          option.value
            ? 'bg-background font-medium text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => {
            filter = option.value;
          }}
          type="button"
        >
          {option.label}
        </button>
      {/each}
    </div>
  </div>

  <div class="relative mt-4">
    {#if timeline.length > 1}
      <div
        aria-hidden="true"
        class="absolute top-2 bottom-2 left-[13px] w-px bg-border"
      ></div>
    {/if}

    <div class="relative space-y-4">
      {#each timeline as entry (entry.kind === "comment" ? entry.comment._id : entry.activity._id)}
        {#if entry.kind === "comment"}
          <IssueCommentBlock
            comment={entry.comment}
            {onDeleteComment}
            onReply={(body) => onAddComment(body, entry.comment._id)}
            {onUpdateComment}
            replies={repliesByParent.get(entry.comment._id) ?? []}
          />
        {:else}
          <IssueActivityRow activity={entry.activity} />
        {/if}
      {:else}
        <p class="text-sm text-muted-foreground">
          {filter === "comments" ? "No comments yet." : "No updates yet."}
        </p>
      {/each}
    </div>
  </div>

  <div class="mt-4">
    <div
      class="rounded-lg border border-input bg-background px-3 py-2 transition-colors focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50"
    >
      <DescriptionEditor
        ariaLabel="New comment"
        bind:this={composer}
        class="min-h-[56px] cursor-text"
        onUpdate={(markdown) => {
          commentDraft = markdown;
        }}
        placeholder="Leave a comment..."
        uploadFile={uploadCommentFile}
      />
    </div>
    <button
      class="mt-2 h-8 rounded-lg bg-primary px-3 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/80 disabled:opacity-50"
      disabled={adding || !commentDraft.trim()}
      onclick={addComment}
      type="button"
    >
      {adding ? "Posting…" : "Comment"}
    </button>
  </div>
</div>
