<script lang="ts">
import Check from "lucide-svelte/icons/check";
import Plus from "lucide-svelte/icons/plus";
import Tag from "lucide-svelte/icons/tag";
import type { CreateLabelInput, Issue, IssueLabel } from "./types";

let {
  issue,
  labels,
  openDropdown,
  onToggleDropdown,
  onToggleLabel,
  onCreateLabel,
}: {
  issue: Issue;
  labels: IssueLabel[];
  openDropdown: string | null;
  onToggleDropdown: (name: string) => void;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
} = $props();

let newLabelName = $state("");
let creatingLabel = $state(false);

function issueHasLabel(labelId: IssueLabel["_id"]) {
  return issue.labels.some((l) => l._id === labelId);
}

async function createLabel() {
  const name = newLabelName.trim();
  if (!name) {
    return;
  }
  creatingLabel = true;
  try {
    await onCreateLabel({ name });
    newLabelName = "";
  } finally {
    creatingLabel = false;
  }
}
</script>

<!-- Labels -->
<div class="flex items-start gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 pt-0.5 text-sm text-zinc-500">
		<Tag class="size-4" />
		<span>Labels</span>
	</div>
	<div class="relative min-w-0 flex-1" data-dropdown>
		<!-- Current labels + trigger -->
		<button
			class="flex flex-wrap items-center gap-1 rounded-md px-2 py-1 text-sm transition hover:bg-white/[0.06]"
			onclick={() => onToggleDropdown("labels")}
			type="button"
		>
			{#if issue.labels.length > 0}
				{#each issue.labels as label (label._id)}
					<span
						class="rounded-full border px-2 py-0.5 text-[11px]"
						style:background-color={`${label.color}18`}
						style:border-color={`${label.color}44`}
						style:color={label.color}
					>
						{label.name}
					</span>
				{/each}
			{:else}
				<span class="text-zinc-500">+ Add labels</span>
			{/if}
		</button>

		{#if openDropdown === "labels"}
			<div class="absolute left-0 top-8 z-50 w-56 rounded-lg border border-white/10 bg-[#1a1b1b] py-1 shadow-xl">
				<!-- Existing labels to toggle -->
				{#if labels.length > 0}
					<div class="max-h-48 overflow-y-auto">
						{#each labels as label (label._id)}
							<button
								class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition hover:bg-white/[0.06] {issueHasLabel(label._id) ? 'text-zinc-100' : 'text-zinc-400'}"
								onclick={() => onToggleLabel(label._id)}
								type="button"
							>
								<span
									class="size-2.5 rounded-full"
									style:background-color={label.color}
								></span>
								<span class="flex-1 truncate">{label.name}</span>
								{#if issueHasLabel(label._id)}
									<Check class="size-3.5 text-amber-400" />
								{/if}
							</button>
						{/each}
					</div>
					<div class="my-1 border-t border-white/[0.06]"></div>
				{/if}

				<!-- Create new label -->
				<form
					class="flex items-center gap-1.5 px-2 py-1.5"
					onsubmit={(e) => { e.preventDefault(); createLabel(); }}
				>
					<input
						bind:value={newLabelName}
						class="h-7 min-w-0 flex-1 rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-amber-400/60"
						placeholder="New label"
					/>
					<button
						class="h-7 rounded-md bg-amber-400 px-2 text-xs font-medium text-black transition hover:bg-amber-300 disabled:opacity-50"
						disabled={creatingLabel || !newLabelName.trim()}
						type="submit"
					>
						<Plus class="size-3.5" />
					</button>
				</form>
			</div>
		{/if}
	</div>
</div>
