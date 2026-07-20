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
	<div class="flex w-[140px] shrink-0 items-center gap-2 pt-0.5 text-sm text-muted-foreground">
		<Tag class="size-4" />
		<span>Labels</span>
	</div>
	<div class="relative min-w-0 flex-1" data-dropdown>
		<!-- Current labels + trigger -->
		<button
			class="flex flex-wrap items-center gap-1 rounded-lg px-2 py-1 text-sm transition-colors hover:bg-muted"
			onclick={() => onToggleDropdown("labels")}
			type="button"
		>
			{#if issue.labels.length > 0}
				{#each issue.labels as label (label._id)}
					<span class="rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 text-meta text-primary">
						{label.name}
					</span>
				{/each}
			{:else}
				<span class="text-muted-foreground">+ Add labels</span>
			{/if}
		</button>

		{#if openDropdown === "labels"}
			<div class="absolute left-0 top-8 z-50 w-56 rounded-lg border border-border bg-popover py-1 text-popover-foreground shadow-md">
				<!-- Existing labels to toggle -->
				{#if labels.length > 0}
					<div class="max-h-48 overflow-y-auto">
						{#each labels as label (label._id)}
							<button
								class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {issueHasLabel(label._id) ? 'text-foreground' : 'text-muted-foreground'}"
								onclick={() => onToggleLabel(label._id)}
								type="button"
							>
								<span class="size-2.5 rounded-full bg-primary"></span>
								<span class="flex-1 truncate">{label.name}</span>
								{#if issueHasLabel(label._id)}
									<Check class="size-3.5 text-primary" />
								{/if}
							</button>
						{/each}
					</div>
					<div class="my-1 border-t border-border"></div>
				{/if}

				<!-- Create new label -->
				<form
					class="flex items-center gap-1.5 px-2 py-1.5"
					onsubmit={(e) => { e.preventDefault(); createLabel(); }}
				>
					<input
						bind:value={newLabelName}
						class="h-7 min-w-0 flex-1 rounded-lg border border-input bg-background px-2 text-xs text-foreground outline-none placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
						placeholder="New label"
					/>
					<button
						aria-label="Create label"
						class="h-7 rounded-lg bg-primary px-2 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/80 disabled:opacity-50"
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
