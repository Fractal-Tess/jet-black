<script lang="ts">
import type { Issue, Sprint } from "$lib/components/issues/types";
import type { Id } from "../../../../../../convex/convex/_generated/dataModel";

let {
  creating,
  issues,
  onAssignIssue,
  onCreate,
  sprints,
}: {
  creating: boolean;
  issues: Issue[];
  onAssignIssue: (
    issueId: Issue["_id"],
    sprintId: Sprint["_id"]
  ) => Promise<void>;
  onCreate: (input: {
    description?: string;
    endDate?: string;
    name: string;
    startDate?: string;
  }) => Promise<void>;
  sprints: Sprint[];
} = $props();

let description = $state("");
let endDate = $state("");
let name = $state("");
let startDate = $state("");

const unassignedIssues = $derived(issues.filter((issue) => !issue.sprintId));

function sprintIssues(sprintId: Id<"sprints">) {
  return issues.filter((issue) => issue.sprintId === sprintId);
}

function completedCount(sprintIssues: Issue[]) {
  return sprintIssues.filter(
    (issue) =>
      issue.state?.type === "completed" || issue.state?.type === "cancelled"
  ).length;
}

function progressPercent(sprintIssues: Issue[]) {
  if (sprintIssues.length === 0) {
    return 0;
  }

  return Math.round((completedCount(sprintIssues) / sprintIssues.length) * 100);
}

function sprintStatus(sprint: Sprint) {
  const today = new Date().toISOString().slice(0, 10);

  if (sprint.endDate && sprint.endDate < today) {
    return "completed";
  }

  if (sprint.startDate && sprint.startDate > today) {
    return "upcoming";
  }

  if (sprint.startDate || sprint.endDate) {
    return "active";
  }

  return "draft";
}

async function createSprint() {
  const nextName = name.trim();

  if (!nextName) {
    return;
  }

  await onCreate({
    description: description.trim() || undefined,
    endDate: endDate || undefined,
    name: nextName,
    startDate: startDate || undefined,
  });
  description = "";
  endDate = "";
  name = "";
  startDate = "";
}

async function assignIssue(event: Event, sprintId: Sprint["_id"]) {
  const select = event.currentTarget as HTMLSelectElement;
  const issueId = select.value as Issue["_id"];

  if (!issueId) {
    return;
  }

  await onAssignIssue(issueId, sprintId);
  select.value = "";
}
</script>

<section class="grid gap-5 xl:grid-cols-[360px_minmax(0,1fr)]">
  <div class="space-y-5">
    <div class="rounded-xl border border-white/10 bg-[#151616]">
      <div class="border-b border-white/[0.06] px-4 py-3">
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Sprints
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">
          New sprint
        </h2>
      </div>

      <form
        class="grid gap-3 p-4"
        onsubmit={(event) => {
          event.preventDefault();
          createSprint();
        }}
      >
        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">Sprint name</span>
          <input
            bind:value={name}
            class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="Sprint 01"
          />
        </label>

        <div class="grid gap-3 sm:grid-cols-2">
          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">Start date</span>
            <input
              bind:value={startDate}
              class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition focus:border-amber-400/60"
              type="date"
            />
          </label>
          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">End date</span>
            <input
              bind:value={endDate}
              class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition focus:border-amber-400/60"
              type="date"
            />
          </label>
        </div>

        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">
            Sprint description
          </span>
          <textarea
            bind:value={description}
            class="min-h-20 w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="Goal, scope, and handoff notes"
          ></textarea>
        </label>

        <button
          class="h-9 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
          disabled={creating || !name.trim()}
          type="submit"
        >
          {creating ? "Creating…" : "Create sprint"}
        </button>
      </form>
    </div>

    <div class="rounded-xl border border-white/10 bg-[#151616] p-4">
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
        Scope pool
      </p>
      <p class="mt-2 text-2xl font-semibold text-zinc-100">
        {unassignedIssues.length}
      </p>
      <p class="text-xs text-zinc-500">tickets not assigned to a sprint</p>
    </div>
  </div>

  <div class="rounded-xl border border-white/10 bg-[#151616]">
    <div
      class="flex items-center justify-between border-b border-white/[0.06] px-4 py-3"
    >
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Timeline
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">
          Project sprints
        </h2>
      </div>
      <span class="rounded-md border border-white/10 px-2 py-1 text-xs text-zinc-500">
        {sprints.length} total
      </span>
    </div>

    <div class="divide-y divide-white/[0.06]">
      {#each sprints as sprint (sprint._id)}
        {@const assignedIssues = sprintIssues(sprint._id)}
        {@const percent = progressPercent(assignedIssues)}
        <article class="p-4">
          <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span
                  class="rounded-full border border-amber-400/15 bg-amber-400/5 px-2 py-0.5 text-[11px] text-amber-300"
                >
                  {sprintStatus(sprint)}
                </span>
                <span class="font-mono text-[10px] text-zinc-600">
                  {sprint.startDate ?? "no start"} → {sprint.endDate ??
                    "no end"}
                </span>
              </div>

              <h3 class="mt-2 text-sm font-semibold text-zinc-100">
                {sprint.name}
              </h3>
              {#if sprint.description}
                <p class="mt-1 text-sm text-zinc-500">
                  {sprint.description}
                </p>
              {/if}

              <div class="mt-4">
                <div class="mb-1 flex items-center justify-between text-xs">
                  <span class="text-zinc-500">
                    {completedCount(assignedIssues)} / {assignedIssues.length}
                    tickets done
                  </span>
                  <span class="font-mono text-zinc-400">{percent}%</span>
                </div>
                <div class="h-1.5 overflow-hidden rounded-full bg-white/10">
                  <div
                    class="h-full rounded-full bg-amber-400"
                    style:width={`${percent}%`}
                  ></div>
                </div>
              </div>

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
                    No tickets in this sprint yet.
                  </span>
                {/each}
              </div>
            </div>

            <label class="block w-full shrink-0 lg:w-64">
              <span class="mb-1 block text-xs text-zinc-500">
                Add ticket to sprint
              </span>
              <select
                class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-xs text-zinc-100 outline-none transition focus:border-amber-400/60"
                onchange={(event) => assignIssue(event, sprint._id)}
              >
                <option value="">Select a ticket…</option>
                {#each unassignedIssues as issue (issue._id)}
                  <option value={issue._id}>
                    {issue.identifier} · {issue.title}
                  </option>
                {/each}
              </select>
            </label>
          </div>
        </article>
      {:else}
        <div class="grid min-h-80 place-items-center p-8 text-center">
          <div>
            <p class="text-sm font-medium text-zinc-300">
              No sprints yet.
            </p>
            <p class="mt-1 max-w-sm text-sm text-zinc-600">
              Create the first sprint to time-box tickets and track delivery
              progress.
            </p>
          </div>
        </div>
      {/each}
    </div>
  </div>
</section>
