<script lang="ts">
import type { Issue, ProjectModuleRecord } from "$lib/components/issues/types";
import type { Id } from "../../../../../../convex/convex/_generated/dataModel";

let {
  creating,
  issues,
  modules,
  onAssignIssue,
  onCreate,
}: {
  creating: boolean;
  issues: Issue[];
  modules: ProjectModuleRecord[];
  onAssignIssue: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onCreate: (input: {
    description?: string;
    name: string;
    status?: ProjectModuleRecord["status"];
    targetDate?: string;
  }) => Promise<void>;
} = $props();

let description = $state("");
let name = $state("");
let status = $state<ProjectModuleRecord["status"]>("planned");
let targetDate = $state("");

const unassignedIssues = $derived(issues.filter((issue) => !issue.moduleId));

function moduleIssues(moduleId: Id<"projectModules">) {
  return issues.filter((issue) => issue.moduleId === moduleId);
}

function completedCount(moduleIssues: Issue[]) {
  return moduleIssues.filter(
    (issue) =>
      issue.state?.type === "completed" || issue.state?.type === "cancelled"
  ).length;
}

function progressPercent(moduleIssues: Issue[]) {
  if (moduleIssues.length === 0) {
    return 0;
  }

  return Math.round((completedCount(moduleIssues) / moduleIssues.length) * 100);
}

function statusLabel(moduleStatus: ProjectModuleRecord["status"]) {
  return moduleStatus.replace("_", " ");
}

async function createModule() {
  const nextName = name.trim();

  if (!nextName) {
    return;
  }

  await onCreate({
    description: description.trim() || undefined,
    name: nextName,
    status,
    targetDate: targetDate || undefined,
  });
  description = "";
  name = "";
  status = "planned";
  targetDate = "";
}

async function assignIssue(event: Event, moduleId: ProjectModuleRecord["_id"]) {
  const select = event.currentTarget as HTMLSelectElement;
  const issueId = select.value as Issue["_id"];

  if (!issueId) {
    return;
  }

  await onAssignIssue(issueId, moduleId);
  select.value = "";
}
</script>

<section class="grid gap-5 xl:grid-cols-[360px_minmax(0,1fr)]">
  <div class="space-y-5">
    <div class="rounded-xl border border-white/10 bg-[#151616]">
      <div class="border-b border-white/[0.06] px-4 py-3">
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Modules
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">
          New module
        </h2>
      </div>

      <form
        class="grid gap-3 p-4"
        onsubmit={(event) => {
          event.preventDefault();
          createModule();
        }}
      >
        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">Module name</span>
          <input
            bind:value={name}
            class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="Billing, Inbox, API"
          />
        </label>

        <div class="grid gap-3 sm:grid-cols-2">
          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">Status</span>
            <select
              bind:value={status}
              class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition focus:border-amber-400/60"
            >
              <option value="backlog">Backlog</option>
              <option value="planned">Planned</option>
              <option value="in_progress">In progress</option>
              <option value="completed">Completed</option>
            </select>
          </label>
          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">Target date</span>
            <input
              bind:value={targetDate}
              class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition focus:border-amber-400/60"
              type="date"
            />
          </label>
        </div>

        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">
            Module description
          </span>
          <textarea
            bind:value={description}
            class="min-h-20 w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="What work belongs in this module?"
          ></textarea>
        </label>

        <button
          class="h-9 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
          disabled={creating || !name.trim()}
          type="submit"
        >
          {creating ? "Creating…" : "Create module"}
        </button>
      </form>
    </div>

    <div class="rounded-xl border border-white/10 bg-[#151616] p-4">
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
        Unsorted
      </p>
      <p class="mt-2 text-2xl font-semibold text-zinc-100">
        {unassignedIssues.length}
      </p>
      <p class="text-xs text-zinc-500">tickets not assigned to a module</p>
    </div>
  </div>

  <div class="rounded-xl border border-white/10 bg-[#151616]">
    <div
      class="flex items-center justify-between border-b border-white/[0.06] px-4 py-3"
    >
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Map
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">
          Project modules
        </h2>
      </div>
      <span class="rounded-md border border-white/10 px-2 py-1 text-xs text-zinc-500">
        {modules.length} total
      </span>
    </div>

    <div class="grid gap-3 p-4 lg:grid-cols-2">
      {#each modules as projectModule (projectModule._id)}
        {@const assignedIssues = moduleIssues(projectModule._id)}
        {@const percent = progressPercent(assignedIssues)}
        <article class="rounded-lg border border-white/[0.08] bg-[#101111] p-4">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <span
                  class="rounded-full border border-amber-400/15 bg-amber-400/5 px-2 py-0.5 text-[11px] capitalize text-amber-300"
                >
                  {statusLabel(projectModule.status)}
                </span>
                {#if projectModule.targetDate}
                  <span class="font-mono text-[10px] text-zinc-600">
                    due {projectModule.targetDate}
                  </span>
                {/if}
              </div>
              <h3 class="mt-2 text-sm font-semibold text-zinc-100">
                {projectModule.name}
              </h3>
            </div>
            <span class="font-mono text-xs text-zinc-500">{percent}%</span>
          </div>

          {#if projectModule.description}
            <p class="mt-2 text-sm text-zinc-500">
              {projectModule.description}
            </p>
          {/if}

          <div class="mt-4">
            <div class="mb-1 flex items-center justify-between text-xs">
              <span class="text-zinc-500">
                {completedCount(assignedIssues)} / {assignedIssues.length}
                tickets done
              </span>
            </div>
            <div class="h-1.5 overflow-hidden rounded-full bg-white/10">
              <div
                class="h-full rounded-full bg-amber-400"
                style:width={`${percent}%`}
              ></div>
            </div>
          </div>

          <label class="mt-4 block">
            <span class="mb-1 block text-xs text-zinc-500">
              Add ticket to module
            </span>
            <select
              class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-xs text-zinc-100 outline-none transition focus:border-amber-400/60"
              onchange={(event) => assignIssue(event, projectModule._id)}
            >
              <option value="">Select a ticket…</option>
              {#each unassignedIssues as issue (issue._id)}
                <option value={issue._id}>
                  {issue.identifier} · {issue.title}
                </option>
              {/each}
            </select>
          </label>

          <div class="mt-4 flex flex-wrap gap-2">
            {#each assignedIssues as issue (issue._id)}
              <span
                class="rounded-md border border-white/10 bg-white/[0.03] px-2 py-1 text-xs text-zinc-300"
              >
                <span class="font-mono text-zinc-600">
                  {issue.identifier}
                </span>
                {issue.title}
              </span>
            {:else}
              <span class="text-xs text-zinc-600">
                No tickets in this module yet.
              </span>
            {/each}
          </div>
        </article>
      {:else}
        <div
          class="col-span-full grid min-h-80 place-items-center p-8 text-center"
        >
          <div>
            <p class="text-sm font-medium text-zinc-300">
              No modules yet.
            </p>
            <p class="mt-1 max-w-sm text-sm text-zinc-600">
              Create modules to group related tickets by product area or
              milestone.
            </p>
          </div>
        </div>
      {/each}
    </div>
  </div>
</section>
