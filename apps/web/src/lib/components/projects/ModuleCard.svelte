<script lang="ts">
import CalendarDays from "lucide-svelte/icons/calendar-days";
import Ellipsis from "lucide-svelte/icons/ellipsis";
import Info from "lucide-svelte/icons/info";
import LayersIcon from "lucide-svelte/icons/layers";
import SquareUser from "lucide-svelte/icons/square-user";
import type {
  ProjectModuleRecord,
  WorkspaceMember,
} from "$lib/components/issues/types";
import { projectModuleDetailHref } from "$lib/routes";
import { moduleStatusBadgeClass, moduleStatusLabel } from "./module-status";

let {
  module: projectModule,
  members = [],
  workspaceSlug,
  projectId,
  onEdit,
}: {
  module: ProjectModuleRecord;
  members?: WorkspaceMember[];
  workspaceSlug: string;
  projectId: string;
  onEdit?: (moduleId: ProjectModuleRecord["_id"]) => void;
} = $props();

const detailHref = $derived(
  projectModuleDetailHref({
    moduleId: projectModule._id,
    projectId,
    workspaceSlug,
  })
);

const progress = $derived(projectModule.progress);

// Mirrors Plane's PROGRESS_STATE_GROUPS_DETAILS (packages/constants/src/state.ts).
const progressSegments = $derived([
  { className: "bg-success", count: progress.completed, name: "Completed" },
  { className: "bg-warning", count: progress.started, name: "Started" },
  { className: "bg-info", count: progress.unstarted, name: "Unstarted" },
  {
    className: "bg-muted-foreground",
    count: progress.backlog,
    name: "Backlog",
  },
]);

// Mirrors Plane's issueCount computation in module-card-item.tsx.
const issueCount = $derived.by(() => {
  if (progress.total === 0) {
    return "0 Work items";
  }
  if (progress.total === progress.completed) {
    return `${progress.total} Work item${progress.total > 1 ? "s" : ""}`;
  }
  return `${progress.completed}/${progress.total} Work items`;
});

const lead = $derived(
  projectModule.leadUserId
    ? (members.find((m) => m.id === projectModule.leadUserId) ?? null)
    : null
);

function segmentWidth(count: number) {
  return `${(count / (progress.total || 1)) * 100}%`;
}
</script>

<article
  class="group relative flex flex-col gap-4 rounded-xl border border-border bg-card p-4 shadow-sm transition-all hover:border-input hover:shadow-md"
>
  <a
    aria-label={`Open ${projectModule.name} module details`}
    class="absolute inset-0 z-10 rounded-xl focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    href={detailHref}
  ></a>

  <div class="relative flex items-center justify-between gap-2">
    <h3
      class="min-w-0 truncate text-sm font-medium text-card-foreground"
      title={projectModule.name}
    >
      {projectModule.name}
    </h3>
    <div class="flex shrink-0 items-center gap-2">
      <span
        class="flex h-6 min-w-20 items-center justify-center rounded px-2 text-center text-meta {moduleStatusBadgeClass(projectModule.status)}"
      >
        {moduleStatusLabel(projectModule.status)}
      </span>
      <Info class="size-4 text-muted-foreground" />
    </div>
  </div>

  <div class="relative flex items-center justify-between">
    <div class="flex items-center gap-1.5">
      <LayersIcon class="size-4 text-muted-foreground" />
      <span class="text-meta text-muted-foreground">{issueCount}</span>
    </div>
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
  </div>

  <div class="relative flex h-2 overflow-hidden rounded bg-muted">
    {#each progressSegments as segment (segment.name)}
      {#if segment.count > 0}
        <div
          class="h-full {segment.className}"
          style:width={segmentWidth(segment.count)}
          title={`${segment.name}: ${segment.count}`}
        ></div>
      {/if}
    {/each}
  </div>

  <div class="relative flex items-center justify-between gap-2">
    <span
      class="flex h-7 items-center gap-1.5 rounded-lg border border-border px-2.5 text-xs text-foreground"
    >
      <CalendarDays class="size-3.5 text-muted-foreground" />
      {#if projectModule.startDate || projectModule.targetDate}
        {projectModule.startDate ?? "Start date"}
        <span class="text-muted-foreground">→</span>
        {projectModule.targetDate ?? "End date"}
      {:else}
        Start date
        <span class="text-muted-foreground">→</span>
        <CalendarDays class="size-3.5 text-muted-foreground" />
        End date
      {/if}
    </span>
    {#if onEdit && projectModule.archivedAt === undefined}
      <button
        aria-label={`Edit ${projectModule.name}`}
        class="relative z-20 grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        onclick={() => onEdit(projectModule._id)}
        title="Edit module"
        type="button"
      >
        <Ellipsis class="size-4" />
      </button>
    {/if}
  </div>
</article>
