<script lang="ts">
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@workspace/ui/components/dropdown-menu";
import Ellipsis from "lucide-svelte/icons/ellipsis";
import DescriptionEditor from "./DescriptionEditor.svelte";
import IssueCommentBlock from "./IssueCommentBlock.svelte";
import { renderMarkdownToHtml } from "./markdown-html";
import { formatFullDate, formatRelativeTime } from "./relative-time";
import type { IssueComment } from "./types";

let {
  comment,
  onDeleteComment,
  onReply,
  onUpdateComment,
  replies = [],
}: {
  comment: IssueComment;
  onDeleteComment: (commentId: IssueComment["_id"]) => Promise<void>;
  onReply?: (body: string) => Promise<void>;
  onUpdateComment: (
    commentId: IssueComment["_id"],
    body: string
  ) => Promise<void>;
  replies?: IssueComment[];
} = $props();

const WHITESPACE_PATTERN = /\s+/;

let editing = $state(false);
let saving = $state(false);
let editor = $state<DescriptionEditor | null>(null);
let replying = $state(false);
let postingReply = $state(false);
let replyEditor = $state<DescriptionEditor | null>(null);

async function postReply() {
  const body = replyEditor?.getMarkdown().trim() ?? "";

  if (!(body && onReply)) {
    return;
  }

  postingReply = true;
  try {
    await onReply(body);
    replying = false;
  } finally {
    postingReply = false;
  }
}

const bodyHtml = $derived(renderMarkdownToHtml(comment.body));

function initials(name: string) {
  const parts = name.trim().split(WHITESPACE_PATTERN);
  const first = parts[0]?.[0] ?? "?";
  const last = parts.length > 1 ? parts.at(-1)?.[0] : "";

  return `${first}${last ?? ""}`.toUpperCase();
}

async function saveEdit() {
  const body = editor?.getMarkdown().trim() ?? "";

  if (!body) {
    return;
  }

  saving = true;
  try {
    await onUpdateComment(comment._id, body);
    editing = false;
  } finally {
    saving = false;
  }
}
</script>

<div class="flex gap-3">
  <span
    class="grid size-7 shrink-0 place-items-center overflow-hidden rounded-full bg-primary text-meta font-medium text-primary-foreground"
  >
    {#if comment.author.image}
      <img alt="" class="size-full object-cover" src={comment.author.image} />
    {:else}
      {initials(comment.author.name)}
    {/if}
  </span>

  <div class="min-w-0 flex-1">
    <div class="flex items-center gap-2">
      <span class="truncate text-xs font-medium text-foreground">
        {comment.author.name}
      </span>
      <span
        class="whitespace-nowrap text-meta text-muted-foreground"
        title={formatFullDate(comment._creationTime)}
      >
        {formatRelativeTime(comment._creationTime)}
      </span>
      {#if comment.editedAt}
        <span class="text-meta text-muted-foreground/70">(edited)</span>
      {/if}

      {#if comment.isAuthor && !editing}
        <DropdownMenu>
          <DropdownMenuTrigger
            aria-label="Comment options"
            class="ml-auto grid size-6 cursor-pointer place-items-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            <Ellipsis class="size-3.5" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" class="w-32">
            <DropdownMenuItem
              class="text-xs"
              onclick={() => {
                editing = true;
              }}
            >
              Edit
            </DropdownMenuItem>
            <DropdownMenuItem
              class="text-xs text-destructive"
              onclick={() => onDeleteComment(comment._id)}
            >
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      {/if}
    </div>

    {#if editing}
      <div class="mt-2 rounded-lg border border-input bg-background px-3 py-2">
        <DescriptionEditor
          ariaLabel="Edit comment"
          bind:this={editor}
          class="min-h-[40px] cursor-text"
          value={comment.body}
        />
      </div>
      <div class="mt-2 flex items-center gap-2">
        <button
          class="h-7 rounded-lg bg-primary px-3 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/80 disabled:opacity-50"
          disabled={saving}
          onclick={saveEdit}
          type="button"
        >
          {saving ? "Saving…" : "Save"}
        </button>
        <button
          class="h-7 rounded-lg border border-input px-3 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          onclick={() => {
            editing = false;
          }}
          type="button"
        >
          Cancel
        </button>
      </div>
    {:else}
      <!-- Safe: HTML is produced by a schema-constrained TipTap renderer, raw HTML is never parsed. -->
      <div class="tiptap mt-1">{@html bodyHtml}</div>
    {/if}

    {#if onReply && !(editing || replying)}
      <button
        class="mt-1 cursor-pointer text-meta text-muted-foreground transition-colors hover:text-foreground"
        onclick={() => {
          replying = true;
        }}
        type="button"
      >
        Reply
      </button>
    {/if}

    {#if replying}
      <div class="mt-2 rounded-lg border border-input bg-background px-3 py-2">
        <DescriptionEditor
          ariaLabel="Reply"
          bind:this={replyEditor}
          class="min-h-[40px] cursor-text"
          placeholder="Write a reply..."
        />
      </div>
      <div class="mt-2 flex items-center gap-2">
        <button
          class="h-7 rounded-lg bg-primary px-3 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/80 disabled:opacity-50"
          disabled={postingReply}
          onclick={postReply}
          type="button"
        >
          {postingReply ? "Posting…" : "Post reply"}
        </button>
        <button
          class="h-7 rounded-lg border border-input px-3 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          onclick={() => {
            replying = false;
          }}
          type="button"
        >
          Cancel
        </button>
      </div>
    {/if}

    {#if replies.length > 0}
      <div class="mt-3 space-y-3 border-l border-border pl-3">
        {#each replies as reply (reply._id)}
          <IssueCommentBlock
            comment={reply}
            {onDeleteComment}
            {onUpdateComment}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>
