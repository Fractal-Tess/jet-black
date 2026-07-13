<script lang="ts">
import type { IntakeIssue } from "$lib/components/issues/types";

let {
  creating,
  intakeIssues,
  onAccept,
  onCreate,
  onDecline,
}: {
  creating: boolean;
  intakeIssues: IntakeIssue[];
  onAccept: (intakeIssue: IntakeIssue) => Promise<void>;
  onCreate: (input: {
    description?: string;
    source?: string;
    title: string;
  }) => Promise<void>;
  onDecline: (intakeIssue: IntakeIssue) => Promise<void>;
} = $props();

let description = $state("");
let source = $state("manual");
let title = $state("");

async function createIntakeIssue() {
  const nextTitle = title.trim();

  if (!nextTitle) {
    return;
  }

  await onCreate({
    description: description.trim() || undefined,
    source: source.trim() || "manual",
    title: nextTitle,
  });
  description = "";
  source = "manual";
  title = "";
}
</script>

<section class="space-y-5">
  <div class="rounded-xl border border-white/10 bg-[#151616]">
    <div class="border-b border-white/[0.06] px-4 py-3">
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
        Intake
      </p>
      <h2 class="mt-1 text-lg font-semibold text-zinc-100">New intake item</h2>
    </div>

    <form
      class="grid gap-3 p-4"
      onsubmit={(event) => {
        event.preventDefault();
        createIntakeIssue();
      }}
    >
      <label class="block">
        <span class="mb-1 block text-xs text-zinc-500">Intake title</span>
        <input
          bind:value={title}
          class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
          placeholder="Describe the incoming request"
        />
      </label>
      <label class="block">
        <span class="mb-1 block text-xs text-zinc-500">Intake source</span>
        <input
          bind:value={source}
          class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
          placeholder="manual, support, sales"
        />
      </label>
      <label class="block">
        <span class="mb-1 block text-xs text-zinc-500">Intake description</span>
        <textarea
          bind:value={description}
          class="min-h-20 w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
          placeholder="Add context before triage"
        ></textarea>
      </label>
      <button
        class="h-9 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
        disabled={creating || !title.trim()}
        type="submit"
      >
        {creating ? "Adding…" : "Add intake item"}
      </button>
    </form>
  </div>

  <div class="rounded-xl border border-white/10 bg-[#151616]">
    <div
      class="flex items-center justify-between border-b border-white/[0.06] px-4 py-3"
    >
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Triage
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">Inbox</h2>
      </div>
      <span class="rounded-md border border-white/10 px-2 py-1 text-xs text-zinc-500">
        {intakeIssues.length} total
      </span>
    </div>

    <div class="divide-y divide-white/[0.06]">
      {#each intakeIssues as intakeIssue (intakeIssue._id)}
        <article class="p-4">
          <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <span
                  class="rounded-full border border-amber-400/15 bg-amber-400/5 px-2 py-0.5 text-[11px] text-amber-300"
                >
                  {intakeIssue.status}
                </span>
                <span class="font-mono text-[10px] text-zinc-600">
                  {intakeIssue.source}
                </span>
              </div>
              <h3 class="mt-2 text-sm font-medium text-zinc-100">
                {intakeIssue.title}
              </h3>
              {#if intakeIssue.description}
                <p class="mt-1 text-sm text-zinc-500">
                  {intakeIssue.description}
                </p>
              {/if}
            </div>

            {#if intakeIssue.status === "pending"}
              <div class="flex shrink-0 gap-2">
                <button
                  class="h-8 rounded-md border border-white/10 px-3 text-xs text-zinc-300 transition hover:bg-white/[0.04]"
                  onclick={() => onDecline(intakeIssue)}
                  type="button"
                >
                  Decline
                </button>
                <button
                  class="h-8 rounded-md bg-amber-400 px-3 text-xs font-semibold text-black transition hover:bg-amber-300"
                  onclick={() => onAccept(intakeIssue)}
                  type="button"
                >
                  Accept
                </button>
              </div>
            {:else if intakeIssue.acceptedIssueId}
              <span class="text-xs text-zinc-600">Accepted as issue</span>
            {/if}
          </div>
        </article>
      {:else}
        <p class="p-8 text-center text-sm text-zinc-600">
          No intake items yet.
        </p>
      {/each}
    </div>
  </div>
</section>
