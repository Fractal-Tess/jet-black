<script lang="ts">
import ArrowLeft from "lucide-svelte/icons/arrow-left";
import Columns3 from "lucide-svelte/icons/columns-3";
import LinkIcon from "lucide-svelte/icons/link";
import List from "lucide-svelte/icons/list";
import Pencil from "lucide-svelte/icons/pencil";
import Plus from "lucide-svelte/icons/plus";
import Search from "lucide-svelte/icons/search";
import Trash2 from "lucide-svelte/icons/trash-2";
import { goto } from "$app/navigation";
import {
  defaultDisplayOptions,
  type IssueDisplayOptions,
} from "$lib/components/issues/display-options";
import IssueDisplayDropdown from "$lib/components/issues/IssueDisplayDropdown.svelte";
import NewIssueModal from "$lib/components/issues/NewIssueModal.svelte";
import type {
  AddAttachmentInput,
  CreateIssueInput,
  CreateLabelInput,
  Issue,
  IssueActivity,
  IssueAttachment,
  IssueComment,
  IssueLabel,
  IssueState,
  ModuleDetail,
  ModuleLink,
  Project,
  ProjectModuleRecord,
  UpdateIssueInput,
  WorkspaceMember,
} from "$lib/components/issues/types";
import { projectModuleHref } from "$lib/routes";
import TicketsView from "../../../routes/workspace/[workspaceSlug]/projects/[projectId]/[module]/components/TicketsView.svelte";

let {
  moduleDetail,
  activeProject,
  issues,
  members,
  states,
  workspaceSlug,
  projectId,
  activities = [],
  attachments,
  comments,
  labels,
  modules = [],
  selectedIssueId,
  selectedSubIssues,
  onRemoveIssue,
  onAssignIssue,
  onCreateIssue,
  onAddLink,
  onUpdateLink,
  onRemoveLink,
  onArchive,
  onDelete,
  onEdit,
  onRestore,
  onMoveIssue,
  onReorderIssue,
  onUpdateIssueFor,
  onAddAttachment,
  onAddComment,
  onUpdateComment,
  onDeleteComment,
  onCreateLabel,
  onCreateModuleForIssue,
  onCreateSubIssue,
  onArchiveIssue,
  onSelectIssue,
  onToggleLabel,
  onUpdateIssue,
}: {
  moduleDetail: NonNullable<ModuleDetail>;
  activeProject: Project;
  issues: Issue[];
  members: WorkspaceMember[];
  states: IssueState[];
  workspaceSlug: string;
  projectId: string;
  activities?: IssueActivity[];
  attachments: IssueAttachment[];
  comments: IssueComment[];
  labels: IssueLabel[];
  modules?: ProjectModuleRecord[];
  selectedIssueId: Issue["_id"] | undefined;
  selectedSubIssues: Issue[];
  onRemoveIssue: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onAssignIssue: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onCreateIssue: (
    moduleId: ProjectModuleRecord["_id"],
    input: CreateIssueInput
  ) => Promise<void>;
  onAddLink: (
    moduleId: ProjectModuleRecord["_id"],
    input: { title: string; url: string }
  ) => Promise<void>;
  onUpdateLink: (
    linkId: ModuleLink["_id"],
    input: { title?: string; url?: string }
  ) => Promise<void>;
  onRemoveLink: (linkId: ModuleLink["_id"]) => Promise<void>;
  onArchive: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onDelete: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onEdit: () => void;
  onRestore: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onReorderIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onUpdateIssueFor: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onAddComment: (
    body: string,
    parentCommentId?: IssueComment["_id"]
  ) => Promise<void>;
  onUpdateComment: (
    commentId: IssueComment["_id"],
    body: string
  ) => Promise<void>;
  onDeleteComment: (commentId: IssueComment["_id"]) => Promise<void>;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateModuleForIssue: (name: string) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onSelectIssue: (issueId: Issue["_id"]) => void;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
} = $props();

let issueView = $state<"board" | "list">("board");
let displayOptions = $state<IssueDisplayOptions>(defaultDisplayOptions());
let issueSearch = $state("");
let createTicketOpen = $state(false);
let creatingTicket = $state(false);
let linkTitle = $state("");
let linkUrl = $state("");
let addingLink = $state(false);
let editingLinkId = $state<string | null>(null);
let editLinkTitle = $state("");
let editLinkUrl = $state("");
let copyStatus = $state("");

const moduleIssues = $derived(
  issues.filter((issue) => moduleDetail.issueIds.includes(issue._id))
);

const searchedModuleIssues = $derived.by(() => {
  const query = issueSearch.trim().toLowerCase();

  if (!query) {
    return moduleIssues;
  }
  return moduleIssues.filter((issue) =>
    issue.title.toLowerCase().includes(query)
  );
});

const unassignedIssues = $derived(
  issues.filter((issue) => !moduleDetail.issueIds.includes(issue._id))
);

const canArchive = $derived(
  moduleDetail.status === "completed" || moduleDetail.status === "cancelled"
);
const isArchived = $derived(moduleDetail.archivedAt !== undefined);
const completedPercent = $derived(
  moduleDetail.progress.total === 0
    ? 0
    : Math.round(
        ((moduleDetail.progress.completed + moduleDetail.progress.cancelled) /
          moduleDetail.progress.total) *
          100
      )
);

function backToModules() {
  goto(
    projectModuleHref({
      module: "modules",
      projectId,
      workspaceSlug,
    })
  );
}

async function handleAddLink(event: Event) {
  event.preventDefault();
  const title = linkTitle.trim();
  const url = linkUrl.trim();

  if (!(title && url)) {
    return;
  }

  addingLink = true;

  try {
    await onAddLink(moduleDetail._id, { title, url });
    linkTitle = "";
    linkUrl = "";
  } finally {
    addingLink = false;
  }
}

async function handleUpdateLink(linkId: string) {
  const title = editLinkTitle.trim();
  const url = editLinkUrl.trim();

  if (!(title && url)) {
    return;
  }

  await onUpdateLink(linkId as ModuleLink["_id"], { title, url });
  editingLinkId = null;
  editLinkTitle = "";
  editLinkUrl = "";
}

function startEditLink(link: ModuleLink) {
  editingLinkId = link._id;
  editLinkTitle = link.title;
  editLinkUrl = link.url;
}

// Issues created inside the module are automatically bound to it.
async function handleQuickCreate(state: IssueState, title: string) {
  const nextTitle = title.trim();

  if (!nextTitle) {
    return;
  }

  await onCreateIssue(moduleDetail._id, {
    priority: "medium",
    stateId: state._id,
    title: nextTitle,
  });
}

async function handleCreateTicket(input: CreateIssueInput) {
  creatingTicket = true;

  try {
    await onCreateIssue(moduleDetail._id, input);
  } finally {
    creatingTicket = false;
  }
}

async function copyLink(url: string) {
  try {
    await navigator.clipboard.writeText(url);
    copyStatus = "Link copied";
  } catch {
    copyStatus = "Unable to copy link";
  }
}
</script>

<div class="flex h-full flex-col gap-4 lg:flex-row">
  <div class="flex min-w-0 flex-1 flex-col">
    <div class="flex items-center justify-between border-b border-border px-4 py-3">
      <div class="flex items-center gap-2">
        <button
          aria-label="Back to modules"
          class="grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          onclick={backToModules}
          type="button"
        >
          <ArrowLeft class="size-4" />
        </button>
        <h2 class="text-sm font-semibold text-foreground">
          {moduleDetail.name}
        </h2>
        <span class="font-mono text-xs text-muted-foreground">
          {moduleIssues.length} issues
        </span>
        {#if !isArchived}
          <button
            aria-label="Edit module"
            class="grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
            onclick={onEdit}
            type="button"
          >
            <Pencil class="size-3.5" />
          </button>
        {/if}
      </div>

      {#if !isArchived}
        <div class="flex items-center gap-2">
          <div class="relative">
            <Search
              class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
            />
            <input
              aria-label="Search issues"
              class="h-8 w-52 rounded-lg border border-input bg-background pl-7 pr-2 text-xs text-foreground outline-none placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
              bind:value={issueSearch}
              placeholder="Search issues…"
            />
          </div>

          <div
            class="inline-flex items-center gap-1 rounded-lg bg-muted p-1"
            role="group"
            aria-label="Layout"
          >
            <button
              aria-label="List"
              aria-pressed={issueView === "list"}
              class="grid h-[22px] w-7 place-items-center rounded-sm transition {issueView ===
              'list'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'}"
              onclick={() => (issueView = "list")}
              title="List layout"
              type="button"
            >
              <List class="size-3.5" />
            </button>
            <button
              aria-label="Board"
              aria-pressed={issueView === "board"}
              class="grid h-[22px] w-7 place-items-center rounded-sm transition {issueView ===
              'board'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'}"
              onclick={() => (issueView = "board")}
              title="Board layout"
              type="button"
            >
              <Columns3 class="size-3.5" />
            </button>
          </div>

          <IssueDisplayDropdown
            {displayOptions}
            onChange={(options) => (displayOptions = options)}
          />

          <button
            class="flex h-8 items-center gap-1.5 rounded-lg bg-primary px-3 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/80"
            onclick={() => (createTicketOpen = true)}
            type="button"
          >
            <Plus class="size-3.5" />
            Add work item
          </button>
        </div>
      {/if}
    </div>

    <div class="flex-1 overflow-hidden px-4 py-3">
      {#if isArchived}
        <div class="divide-y divide-border rounded-lg border border-border bg-card">
          {#each moduleIssues as issue (issue._id)}
            <div class="flex items-center gap-3 px-4 py-3">
              <span class="font-mono text-meta text-muted-foreground">
                {issue.identifier}
              </span>
              <span class="min-w-0 flex-1 truncate text-sm text-foreground">
                {issue.title}
              </span>
              <span class="text-xs text-muted-foreground">
                {issue.state?.name ?? "Unknown state"}
              </span>
            </div>
          {:else}
            <p class="p-6 text-sm text-muted-foreground">
              No work items were assigned to this module.
            </p>
          {/each}
        </div>
      {:else}
        <TicketsView
          {activeProject}
          {activities}
          {attachments}
          {comments}
          {displayOptions}
          filteredIssues={searchedModuleIssues}
          {issueView}
          {labels}
          {members}
          {modules}
          {onAddAttachment}
          {onAddComment}
          {onUpdateComment}
          {onDeleteComment}
          {onArchiveIssue}
          {onCreateLabel}
    {onCreateModuleForIssue}
          {onCreateSubIssue}
          {onMoveIssue}
          onQuickCreateIssue={handleQuickCreate}
          {onReorderIssue}
          {onSelectIssue}
          {onToggleLabel}
          {onUpdateIssue}
          {onUpdateIssueFor}
          {selectedIssueId}
          {selectedSubIssues}
          {states}
          {workspaceSlug}
        />
      {/if}
    </div>
  </div>

  <aside class="w-full shrink-0 border-t border-border lg:w-80 lg:border-t-0 lg:border-l">
    <div class="space-y-5 p-4">
      <div>
        <h3 class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Details
        </h3>
        <div class="mt-3 space-y-3 text-sm">
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">Status</span>
            <span class="capitalize text-foreground">
              {moduleDetail.status === "in_progress" ? "In progress" : moduleDetail.status}
            </span>
          </div>
          {#if moduleDetail.startDate}
            <div class="flex items-center justify-between">
              <span class="text-muted-foreground">Start</span>
              <span class="font-mono text-xs text-foreground">
                {moduleDetail.startDate}
              </span>
            </div>
          {/if}
          {#if moduleDetail.targetDate}
            <div class="flex items-center justify-between">
              <span class="text-muted-foreground">Target</span>
              <span class="font-mono text-xs text-foreground">
                {moduleDetail.targetDate}
              </span>
            </div>
          {/if}
          {#if moduleDetail.leadUserId}
            <div class="flex items-center justify-between">
              <span class="text-muted-foreground">Lead</span>
              <span class="text-foreground">
                {members.find((m) => m.id === moduleDetail.leadUserId)?.name ?? "—"}
              </span>
            </div>
          {/if}
        </div>
      </div>

      <div>
        <div class="flex items-center justify-between">
          <h3 class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
            Progress
          </h3>
          <span class="font-mono text-xs text-foreground">{completedPercent}%</span>
        </div>
        <div class="mt-3 flex h-1.5 overflow-hidden rounded-full bg-muted">
          {#each [
            ["backlog", moduleDetail.progress.backlog, "bg-muted-foreground"],
            ["unstarted", moduleDetail.progress.unstarted, "bg-info"],
            ["started", moduleDetail.progress.started, "bg-warning"],
            ["completed", moduleDetail.progress.completed, "bg-success"],
            ["cancelled", moduleDetail.progress.cancelled, "bg-destructive"],
          ] as segment (segment[0])}
            {#if Number(segment[1]) > 0}
              <span
                class={String(segment[2])}
                style:width={`${(Number(segment[1]) / moduleDetail.progress.total) * 100}%`}
                title={`${segment[0]}: ${segment[1]}`}
              ></span>
            {/if}
          {/each}
        </div>
        <p class="mt-2 text-xs text-muted-foreground">
          {moduleDetail.progress.completed} completed · {moduleDetail.progress.started} in progress · {moduleDetail.progress.total} total
        </p>
      </div>

      {#if moduleDetail.memberIds.length > 0}
        <div>
          <h3 class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
            Members
          </h3>
          <div class="mt-3 flex flex-wrap gap-1.5">
            {#each moduleDetail.memberIds as memberId (memberId)}
              <span class="rounded-md border border-border bg-card px-2 py-1 text-xs text-foreground">
                {members.find((member) => member.id === memberId)?.name ?? "Former member"}
              </span>
            {/each}
          </div>
        </div>
      {/if}

      <div>
        <h3 class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Links
        </h3>
        <div class="mt-3 space-y-2">
          {#each moduleDetail.links as link (link._id)}
            <div class="flex items-center gap-2 rounded-md border border-border bg-card p-2">
              <LinkIcon class="size-3.5 shrink-0 text-muted-foreground" />
              {#if editingLinkId === link._id}
                <form
                  class="flex min-w-0 flex-1 flex-col gap-1"
                  onsubmit={(e) => {
                    e.preventDefault();
                    handleUpdateLink(link._id);
                  }}
                >
                  <input
                    aria-label="Link title"
                    class="w-full rounded bg-transparent text-xs text-foreground outline-none"
                    bind:value={editLinkTitle}
                  />
                  <input
                    aria-label="Link URL"
                    class="w-full rounded bg-transparent text-meta text-muted-foreground outline-none"
                    bind:value={editLinkUrl}
                  />
                  <div class="flex gap-1">
                    <button
                      class="rounded bg-primary px-2 py-0.5 text-meta text-primary-foreground"
                      type="submit"
                    >
                      Save
                    </button>
                    <button
                      class="rounded bg-muted px-2 py-0.5 text-meta text-muted-foreground"
                      onclick={() => {
                        editingLinkId = null;
                        editLinkTitle = "";
                        editLinkUrl = "";
                      }}
                      type="button"
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              {:else}
                <div class="min-w-0 flex-1">
                  <a
                    class="block truncate text-xs text-foreground hover:text-primary"
                    href={link.url}
                    rel="noopener noreferrer"
                    target="_blank"
                  >
                    {link.title}
                  </a>
                    <span class="block truncate font-mono text-meta text-muted-foreground">
                    {link.url}
                  </span>
                </div>
                <div class="flex items-center gap-0.5">
                  <button
                    aria-label={`Copy link ${link.title}`}
                    class="grid size-6 place-items-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                    onclick={() => copyLink(link.url)}
                    title="Copy URL"
                    type="button"
                  >
                    <LinkIcon class="size-3" />
                  </button>
                  {#if !isArchived}
                    <button
                      aria-label={`Edit link ${link.title}`}
                      class="grid size-6 place-items-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                      onclick={() => startEditLink(link)}
                      title="Edit"
                      type="button"
                    >
                      <Plus class="size-3 rotate-45" />
                    </button>
                    <button
                      aria-label={`Delete link ${link.title}`}
                      class="grid size-6 place-items-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-destructive"
                      onclick={() => onRemoveLink(link._id)}
                      title="Delete"
                      type="button"
                    >
                      <Trash2 class="size-3" />
                    </button>
                  {/if}
                </div>
              {/if}
            </div>
          {:else}
            <p class="text-xs text-muted-foreground">No links yet.</p>
          {/each}

          {#if !isArchived}
            <form class="flex flex-col gap-1.5" onsubmit={handleAddLink}>
              <input
                aria-label="New link title"
                class="rounded border border-border bg-card px-2 py-1 text-xs text-foreground outline-none placeholder:text-muted-foreground"
                bind:value={linkTitle}
                placeholder="Link title"
              />
              <input
                aria-label="New link URL"
                class="rounded border border-border bg-card px-2 py-1 text-xs text-foreground outline-none placeholder:text-muted-foreground"
                bind:value={linkUrl}
                placeholder="https://…"
              />
              <button
                aria-label="Add link"
                class="flex items-center gap-1 rounded bg-primary px-2 py-1 text-meta font-semibold text-primary-foreground transition-colors hover:bg-primary/80 disabled:opacity-50"
                disabled={addingLink || !linkTitle.trim() || !linkUrl.trim()}
                type="submit"
              >
                <Plus class="size-3" />
                Add link
              </button>
            </form>
          {/if}
          <p aria-live="polite" class="sr-only">{copyStatus}</p>
        </div>
      </div>

      <div>
        <h3 class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Work items
        </h3>
        <div class="mt-3 space-y-2">
          {#if !isArchived && unassignedIssues.length > 0}
            <div class="space-y-1">
              <p class="text-meta text-muted-foreground">Add existing</p>
              <div class="max-h-40 overflow-y-auto space-y-1">
                {#each unassignedIssues.slice(0, 20) as issue (issue._id)}
                  <button
                    aria-label={`Add ${issue.title} to module`}
                    class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-xs text-foreground transition-colors hover:bg-muted"
                    onclick={() => onAssignIssue(issue._id, moduleDetail._id)}
                    type="button"
                  >
                    <span class="font-mono text-meta text-muted-foreground">
                      {issue.identifier}
                    </span>
                    <span class="truncate">{issue.title}</span>
                    <Plus class="ml-auto size-3 shrink-0 text-muted-foreground" />
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          {#if moduleIssues.length > 0}
            <div class="space-y-1">
              <p class="text-meta text-muted-foreground">Assigned</p>
              <div class="space-y-1">
                {#each moduleIssues as issue (issue._id)}
                  <div class="flex items-center gap-2 rounded px-2 py-1 text-xs text-foreground">
                    <span class="font-mono text-meta text-muted-foreground">
                      {issue.identifier}
                    </span>
                    <span class="truncate">{issue.title}</span>
                    {#if !isArchived}
                      <button
                        aria-label={`Remove ${issue.title} from module`}
                        class="ml-auto grid size-5 place-items-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-destructive"
                        onclick={() =>
                          onRemoveIssue(issue._id, moduleDetail._id)}
                        type="button"
                      >
                        <Trash2 class="size-3" />
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {:else}
            <p class="text-xs text-muted-foreground">No work items assigned.</p>
          {/if}
        </div>
      </div>

      <div class="space-y-2">
        {#if moduleDetail.archivedAt !== undefined}
          <button
            aria-label="Restore module"
            class="w-full rounded border border-success/30 bg-card px-3 py-2 text-xs text-success transition-colors hover:bg-success/10"
            onclick={() => onRestore(moduleDetail._id)}
            type="button"
          >
            Restore module
          </button>
        {:else}
          {#if !canArchive}
            <p class="text-xs leading-relaxed text-muted-foreground">
              Only completed or cancelled modules can be archived.
            </p>
          {/if}
          <button
            aria-label="Archive module"
            class="w-full rounded border border-border bg-card px-3 py-2 text-xs text-foreground transition-colors hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
            disabled={!canArchive}
            onclick={() => onArchive(moduleDetail._id)}
            type="button"
          >
            Archive module
          </button>
        {/if}
        <button
          aria-label="Delete module"
          class="w-full rounded border border-destructive/30 bg-card px-3 py-2 text-xs text-destructive transition hover:bg-destructive/10"
          onclick={() => onDelete(moduleDetail._id)}
          type="button"
        >
          Delete module
        </button>
      </div>
    </div>
  </aside>
</div>

{#if createTicketOpen}
  <NewIssueModal
    creating={creatingTicket}
    isOpen={createTicketOpen}
    {members}
    onClose={() => (createTicketOpen = false)}
    onCreate={(input) => handleCreateTicket(input)}
    project={activeProject}
    {states}
  />
{/if}
