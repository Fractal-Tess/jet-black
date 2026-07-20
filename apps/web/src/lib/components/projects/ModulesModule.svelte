<script lang="ts">
import Archive from "lucide-svelte/icons/archive";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import CalendarRange from "lucide-svelte/icons/calendar-range";
import Check from "lucide-svelte/icons/check";
import Columns3 from "lucide-svelte/icons/columns-3";
import List from "lucide-svelte/icons/list";
import Pencil from "lucide-svelte/icons/pencil";
import Plus from "lucide-svelte/icons/plus";
import SquareUser from "lucide-svelte/icons/square-user";
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
  ModuleDetail as ModuleDetailType,
  ModuleLink,
  Project,
  ProjectModuleRecord,
  UpdateIssueInput,
  UpdateProjectModuleInput,
  WorkspaceMember,
} from "$lib/components/issues/types";
import {
  projectModuleDetailHref,
  projectModuleHref,
  workspaceHref,
  workspaceProjectsHref,
} from "$lib/routes";
import ModuleCard from "./ModuleCard.svelte";
import ModuleDetail from "./ModuleDetail.svelte";
import ModuleModal from "./ModuleModal.svelte";
import { moduleStatusBadgeClass, moduleStatusLabel } from "./module-status";

let {
  activeProject,
  creating,
  issues,
  modules,
  archivedModules,
  moduleDetail,
  moduleDetailLoading,
  routeModuleId,
  workspaceSlug,
  members = [],
  states = [],
  activities = [],
  attachments,
  comments,
  labels,
  selectedIssueId,
  selectedSubIssues,
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
  onArchive,
  onCreate,
  onCreateIssue,
  onDelete,
  onAddLink,
  onUpdateLink,
  onRemoveLink,
  onRemoveIssue,
  onRestore,
  onUpdate,
  onAssignIssue,
  onMoveIssue,
  onReorderIssue,
  onUpdateIssueFor,
}: {
  activeProject: Project;
  creating: boolean;
  issues: Issue[];
  modules: ProjectModuleRecord[];
  archivedModules: ProjectModuleRecord[];
  moduleDetail: ModuleDetailType | null;
  moduleDetailLoading: boolean;
  routeModuleId: string | undefined;
  workspaceSlug: string;
  members?: WorkspaceMember[];
  states?: IssueState[];
  activities?: IssueActivity[];
  attachments: IssueAttachment[];
  comments: IssueComment[];
  labels: IssueLabel[];
  selectedIssueId: Issue["_id"] | undefined;
  selectedSubIssues: Issue[];
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
  onArchive: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onCreate: (input: {
    description?: string;
    leadUserId?: string;
    memberIds?: string[];
    name: string;
    startDate?: string;
    status?: ProjectModuleRecord["status"];
    targetDate?: string;
  }) => Promise<void>;
  onCreateIssue: (
    moduleId: ProjectModuleRecord["_id"],
    input: CreateIssueInput
  ) => Promise<void>;
  onDelete: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onAddLink: (
    moduleId: ProjectModuleRecord["_id"],
    input: { title: string; url: string }
  ) => Promise<void>;
  onUpdateLink: (
    linkId: ModuleLink["_id"],
    input: { title?: string; url?: string }
  ) => Promise<void>;
  onRemoveLink: (linkId: ModuleLink["_id"]) => Promise<void>;
  onRemoveIssue: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onRestore: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onUpdate: (
    moduleId: ProjectModuleRecord["_id"],
    input: UpdateProjectModuleInput
  ) => Promise<void>;
  onAssignIssue: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
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
} = $props();

let showArchived = $state(false);
let layout = $state<"list" | "board" | "gantt">("board");
let createDialogOpen = $state(false);
let editModuleId = $state<string | null>(null);

const allModules = $derived(showArchived ? archivedModules : modules);

const filteredModules = $derived(
  allModules.toSorted((a, b) => b._creationTime - a._creationTime)
);

const editingModule = $derived(
  editModuleId ? (modules.find((m) => m._id === editModuleId) ?? null) : null
);

const activeDetail = $derived(
  routeModuleId &&
    moduleDetail &&
    moduleDetail._id === routeModuleId &&
    moduleDetail.projectId === activeProject._id
    ? moduleDetail
    : null
);

const DAY_MS = 86_400_000;
const datedModules = $derived(
  filteredModules.filter((projectModule) =>
    Boolean(projectModule.startDate || projectModule.targetDate)
  )
);
const ganttStart = $derived(
  datedModules.length > 0
    ? Math.min(
        ...datedModules.map((projectModule) =>
          Date.parse(
            `${projectModule.startDate ?? projectModule.targetDate}T00:00:00Z`
          )
        )
      )
    : 0
);
const ganttEnd = $derived(
  datedModules.length > 0
    ? Math.max(
        ...datedModules.map((projectModule) =>
          Date.parse(
            `${projectModule.targetDate ?? projectModule.startDate}T00:00:00Z`
          )
        )
      )
    : 0
);

function ganttPosition(projectModule: ProjectModuleRecord) {
  const start = Date.parse(
    `${projectModule.startDate ?? projectModule.targetDate}T00:00:00Z`
  );
  const end = Date.parse(
    `${projectModule.targetDate ?? projectModule.startDate}T00:00:00Z`
  );
  const span = Math.max(ganttEnd - ganttStart + DAY_MS, DAY_MS);

  return {
    left: `${((start - ganttStart) / span) * 100}%`,
    width: `${Math.max(((end - start + DAY_MS) / span) * 100, 2)}%`,
  };
}

async function handleCreate(input: {
  description?: string | null;
  leadUserId?: string | null;
  memberIds?: string[];
  name: string;
  startDate?: string | null;
  status: ProjectModuleRecord["status"];
  targetDate?: string | null;
}) {
  await onCreate({
    description: input.description ?? undefined,
    leadUserId: input.leadUserId ?? undefined,
    memberIds: input.memberIds,
    name: input.name,
    startDate: input.startDate ?? undefined,
    status: input.status,
    targetDate: input.targetDate ?? undefined,
  });
  createDialogOpen = false;
}

async function handleUpdate(
  moduleId: string,
  input: Parameters<typeof onUpdate>[1]
) {
  await onUpdate(moduleId as ProjectModuleRecord["_id"], input);
  editModuleId = null;
}

// Mirrors Plane's module-list-item.tsx completion percentage.
function moduleCompletion(projectModule: ProjectModuleRecord): number {
  const { cancelled, completed, total } = projectModule.progress;
  if (total === 0) {
    return 0;
  }
  return Math.floor(((completed + cancelled) / total) * 100);
}

function moduleLead(
  projectModule: ProjectModuleRecord
): WorkspaceMember | null {
  if (!projectModule.leadUserId) {
    return null;
  }
  return members.find((m) => m.id === projectModule.leadUserId) ?? null;
}
</script>

{#if routeModuleId && activeDetail}
  <ModuleDetail
    moduleDetail={activeDetail}
    {activeProject}
    {issues}
    {members}
    {states}
    {workspaceSlug}
    projectId={activeProject._id}
    {activities}
    {attachments}
    {comments}
    {labels}
    {modules}
    {selectedIssueId}
    {selectedSubIssues}
    onArchive={onArchive}
    onAssignIssue={onAssignIssue}
    onDelete={onDelete}
    onEdit={() => (editModuleId = activeDetail._id)}
    onRemoveIssue={onRemoveIssue}
    onRestore={onRestore}
    onCreateIssue={onCreateIssue}
    onAddLink={onAddLink}
    onUpdateLink={onUpdateLink}
    onRemoveLink={onRemoveLink}
    onMoveIssue={onMoveIssue}
    onReorderIssue={onReorderIssue}
    onUpdateIssueFor={onUpdateIssueFor}
    {onAddAttachment}
    {onAddComment}
    {onUpdateComment}
    {onDeleteComment}
    {onCreateLabel}
    {onCreateModuleForIssue}
    {onCreateSubIssue}
    {onArchiveIssue}
    {onSelectIssue}
    {onToggleLabel}
    {onUpdateIssue}
  />
{:else if routeModuleId}
  <div class="grid min-h-[28rem] place-items-center rounded-lg border border-border bg-card p-8 text-center">
    <div>
      <p class="text-sm font-medium text-foreground">
        {moduleDetailLoading ? "Loading module…" : "Module not found."}
      </p>
      <p class="mt-1 text-sm text-muted-foreground">
        {moduleDetailLoading
          ? "Fetching the latest module details."
          : "This module does not exist in the current project or you cannot access it."}
      </p>
      {#if !moduleDetailLoading}
        <a
      class="mt-4 inline-flex h-8 items-center rounded-lg bg-primary px-3 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/80"
          href={projectModuleHref({
            module: "modules",
            projectId: activeProject._id,
            workspaceSlug,
          })}
        >
          Back to modules
        </a>
      {/if}
    </div>
  </div>
{:else}
  <div class="flex h-full flex-col">
    <div class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-3 sm:px-8">
      <div class="flex min-w-0 items-center gap-2.5">
      <div class="flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground">
        <a class="transition-colors hover:text-foreground" href={workspaceHref(workspaceSlug)}>
            Home
          </a>
        <span class="text-muted-foreground">/</span>
        <a class="transition-colors hover:text-foreground" href={workspaceProjectsHref(workspaceSlug)}>
            Projects
          </a>
        <span class="text-muted-foreground">/</span>
          <a
          class="max-w-40 truncate transition-colors hover:text-foreground"
            href={projectModuleHref({
              module: "tickets",
              projectId: activeProject._id,
              workspaceSlug,
            })}
          >
            {activeProject.name}
          </a>
        <span class="text-muted-foreground">/</span>
        <span class="font-medium text-foreground">Modules</span>
        </div>
        {#if allModules.length > 0}
      <span class="inline-flex items-center rounded-md border border-border px-2 py-0.5 text-meta tabular-nums text-muted-foreground">
            {allModules.length}
          </span>
        {/if}
      </div>

      <div class="flex items-center gap-2">
        <div
      class="inline-flex items-center gap-1 rounded-lg bg-muted p-1"
          role="group"
          aria-label="Layout"
        >
          <button
            aria-label="List"
            aria-pressed={layout === "list"}
            class="grid h-[22px] w-7 place-items-center rounded-sm transition {layout ===
            'list'
          ? 'bg-background text-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => (layout = "list")}
            title="List layout"
            type="button"
          >
            <List class="size-3.5" />
          </button>
          <button
            aria-label="Board"
            aria-pressed={layout === "board"}
            class="grid h-[22px] w-7 place-items-center rounded-sm transition {layout ===
            'board'
          ? 'bg-background text-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => (layout = "board")}
            title="Board layout"
            type="button"
          >
            <Columns3 class="size-3.5" />
          </button>
          <button
            aria-label="Gantt"
            aria-pressed={layout === "gantt"}
            class="grid h-[22px] w-7 place-items-center rounded-sm transition {layout ===
            'gantt'
          ? 'bg-background text-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => (layout = "gantt")}
            title="Gantt layout"
            type="button"
          >
            <CalendarRange class="size-3.5" />
          </button>
        </div>

        <button
          aria-label={showArchived ? "Show active" : "Show archived"}
          aria-pressed={showArchived}
      class="inline-flex h-8 items-center gap-1.5 rounded-lg border border-border bg-card px-3 text-xs text-foreground transition-colors hover:bg-muted {showArchived
      ? 'bg-muted'
            : ''}"
          onclick={() => (showArchived = !showArchived)}
          type="button"
        >
          <Archive class="size-3.5" />
          {showArchived ? "Archived" : "Active"}
        </button>

        <button
          aria-label="New module"
      class="inline-flex h-8 items-center gap-1.5 rounded-lg bg-primary px-3 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/80"
          onclick={() => (createDialogOpen = true)}
          type="button"
        >
          <Plus class="size-3.5" />
          Module
        </button>
      </div>
    </div>

    <div class="px-5 py-4 sm:px-8">
      {#if filteredModules.length > 0}
      {#if layout === "board"}
        <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {#each filteredModules as projectModule (projectModule._id)}
            <ModuleCard
              {members}
              module={projectModule}
              onEdit={showArchived ? undefined : (moduleId) => (editModuleId = moduleId)}
              projectId={activeProject._id}
              {workspaceSlug}
            />
          {/each}
        </div>
      {:else if layout === "list"}
        <div class="rounded-xl border border-border bg-card shadow-sm">
          {#each filteredModules as projectModule (projectModule._id)}
            {@const completion = moduleCompletion(projectModule)}
            {@const lead = moduleLead(projectModule)}
            <div
              class="group flex items-center justify-between gap-3 border-b border-border px-4 py-2.5 transition-colors last:border-b-0 hover:bg-muted/50"
            >
              <div class="flex min-w-0 items-center gap-3">
                <span
                  class="relative grid size-[30px] shrink-0 place-items-center"
                  title={`${completion}% complete`}
                >
                  <svg
                    aria-hidden="true"
                    class="absolute inset-0 -rotate-90"
                    viewBox="0 0 30 30"
                  >
                    <circle
                      cx="15"
                      cy="15"
                      fill="none"
                      r="12"
                      class="text-border"
                      stroke="currentColor"
                      stroke-width="3"
                    />
                    <circle
                      cx="15"
                      cy="15"
                      fill="none"
                      r="12"
                      class="text-primary"
                      stroke="currentColor"
                      stroke-dasharray={`${(completion / 100) * 2 * Math.PI * 12} ${2 * Math.PI * 12}`}
                      stroke-linecap="round"
                      stroke-width="3"
                    />
                  </svg>
                  {#if completion === 100}
                    <Check class="size-3 text-primary" />
                  {:else}
                    <span class="text-meta text-muted-foreground">{completion}%</span>
                  {/if}
                </span>
                <a
                  class="min-w-0 truncate text-sm font-medium text-foreground hover:text-primary"
                  href={projectModuleDetailHref({
                    moduleId: projectModule._id,
                    projectId: activeProject._id,
                    workspaceSlug,
                  })}
                >
                  {projectModule.name}
                </a>
              </div>

              <div class="flex shrink-0 items-center gap-2.5">
                <span
                  class="flex h-6 items-center gap-1.5 rounded-md border border-border px-2 text-meta text-muted-foreground"
                >
                  <CalendarDays class="size-3 text-muted-foreground" />
                  {#if projectModule.startDate && projectModule.targetDate}
                    {projectModule.startDate} → {projectModule.targetDate}
                  {:else if projectModule.startDate}
                    from {projectModule.startDate}
                  {:else if projectModule.targetDate}
                    due {projectModule.targetDate}
                  {:else}
                    No dates
                  {/if}
                </span>
                <span
                  class="flex h-6 w-20 items-center justify-center rounded-sm text-center text-meta {moduleStatusBadgeClass(projectModule.status)}"
                >
                  {moduleStatusLabel(projectModule.status)}
                </span>
                {#if lead}
                  <span
                    class="inline-flex size-5 items-center justify-center overflow-hidden rounded-full bg-primary text-meta font-medium text-primary-foreground"
                    title={lead.name ?? lead.email}
                  >
                    {#if lead.image}
                      <img alt="" class="size-full object-cover" src={lead.image} />
                    {:else}
                      {(lead.name ?? lead.email).charAt(0).toUpperCase()}
                    {/if}
                  </span>
                {:else}
                  <span title="No lead">
                    <SquareUser class="size-4 text-muted-foreground" />
                  </span>
                {/if}
                {#if showArchived}
                  <span class="size-6"></span>
                {:else}
                  <button
                    aria-label={`Edit ${projectModule.name}`}
                    class="grid size-6 place-items-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                    onclick={() => (editModuleId = projectModule._id)}
                    type="button"
                  >
                    <Pencil class="size-3.5" />
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {:else if datedModules.length > 0}
        <div class="overflow-hidden rounded-lg border border-border bg-card">
          <div class="grid grid-cols-[minmax(11rem,16rem)_minmax(32rem,1fr)] border-b border-border bg-muted/30 px-4 py-2 text-meta font-medium uppercase tracking-wider text-muted-foreground">
            <span>Module</span>
            <span class="flex justify-between">
              <span>{new Date(ganttStart).toISOString().slice(0, 10)}</span>
              <span>{new Date(ganttEnd).toISOString().slice(0, 10)}</span>
            </span>
          </div>
          {#each datedModules as projectModule (projectModule._id)}
            {@const position = ganttPosition(projectModule)}
            <div class="grid min-h-12 grid-cols-[minmax(11rem,16rem)_minmax(32rem,1fr)] items-center border-b border-border px-4 py-2 last:border-b-0">
              <a
                    class="truncate pr-4 text-sm font-medium text-foreground transition-colors hover:text-primary"
                href={projectModuleDetailHref({
                  moduleId: projectModule._id,
                  projectId: activeProject._id,
                  workspaceSlug,
                })}
              >
                {projectModule.name}
              </a>
                <div class="relative h-7 rounded bg-muted/30">
                <a
                  aria-label={`Open ${projectModule.name} module details`}
                    class="absolute top-1 h-5 min-w-3 rounded bg-primary/80 shadow-sm transition-colors hover:bg-primary"
                  href={projectModuleDetailHref({
                    moduleId: projectModule._id,
                    projectId: activeProject._id,
                    workspaceSlug,
                  })}
                  style:left={position.left}
                  style:width={position.width}
                  title={`${projectModule.startDate ?? projectModule.targetDate} – ${projectModule.targetDate ?? projectModule.startDate}`}
                ></a>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="grid min-h-64 place-items-center rounded-lg border border-border bg-card p-8 text-center">
          <div>
            <p class="text-sm font-medium text-foreground">No dated modules.</p>
            <p class="mt-1 text-sm text-muted-foreground">
              Add a start or target date to place modules on the timeline.
            </p>
          </div>
        </div>
      {/if}
      {:else}
      <div class="grid min-h-80 place-items-center rounded-lg border border-border bg-card p-8 text-center">
        <div>
          <p class="text-sm font-medium text-secondary-foreground">
            {showArchived ? "No archived modules." : "No modules yet."}
          </p>
          <p class="mt-1 max-w-sm text-sm text-muted-foreground">
            {showArchived
              ? "Archived modules will appear here."
              : "Create modules to group related tickets by product area or milestone."}
          </p>
        </div>
      </div>
      {/if}
    </div>
  </div>
{/if}

<ModuleModal
  isOpen={createDialogOpen}
  {members}
  onClose={() => (createDialogOpen = false)}
  onSave={handleCreate}
  project={activeProject}
  saving={creating}
/>

{#if editingModule}
  <ModuleModal
    isOpen={Boolean(editingModule)}
    {members}
    onClose={() => (editModuleId = null)}
    onSave={(input) => handleUpdate(editingModule._id, input)}
    project={activeProject}
    value={{
      description: editingModule.description,
      leadUserId: editingModule.leadUserId,
      memberIds: editingModule.memberIds,
      name: editingModule.name,
      startDate: editingModule.startDate,
      status: editingModule.status,
      targetDate: editingModule.targetDate,
    }}
  />
{/if}
