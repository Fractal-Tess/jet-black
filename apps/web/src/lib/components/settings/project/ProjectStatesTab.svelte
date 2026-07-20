<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { useAuth, useMutation, useQuery } from "convex-svelte";
import Plus from "lucide-svelte/icons/plus";
import type { IssueState } from "$lib/components/issues/types";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";
import StateRow from "./StateRow.svelte";
import {
  STATE_COLOR_PALETTE,
  STATE_TYPE_ORDER,
  type StateType,
} from "./state-colors";

let {
  project,
  role,
}: {
  project: { _id: string };
  role: WorkspaceRole;
} = $props();

const canManage = $derived(role !== "guest");
const auth = useAuth();
const projectId = $derived(project._id as Id<"projects">);

const statesQuery = useQuery(api.queries.workspaces.statesForProject, () =>
  auth.isAuthenticated ? { projectId } : "skip"
);

const createState = useMutation(api.mutations.states.create);
const updateState = useMutation(api.mutations.states.update);
const setDefaultState = useMutation(api.mutations.states.setDefault);
const reorderState = useMutation(api.mutations.states.reorder);
const removeState = useMutation(api.mutations.states.remove);

const states = $derived(statesQuery.data ?? []);

function groupFor(type: StateType): IssueState[] {
  return states
    .filter((state) => state.type === type)
    .sort((a, b) => a.position - b.position);
}

let creatingType = $state<StateType | null>(null);
let newName = $state("");
let newColor = $state(STATE_COLOR_PALETTE[0]);
let error = $state("");

function startCreate(type: StateType) {
  creatingType = type;
  newName = "";
  newColor = STATE_COLOR_PALETTE[0];
  error = "";
}

async function submitCreate() {
  if (!creatingType) {
    return;
  }

  const name = newName.trim();
  if (!name) {
    error = "Name is required";
    return;
  }

  try {
    await createState({ color: newColor, name, projectId, type: creatingType });
    creatingType = null;
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not create state.";
  }
}

async function swap(a: IssueState, b: IssueState) {
  await reorderState({ position: b.position, stateId: a._id });
  await reorderState({ position: a.position, stateId: b._id });
}
</script>

<SettingsHeading
  title="States"
  description="Organize workflow states within your project."
/>

{#if error}
  <p class="mt-4 text-sm text-destructive" role="alert">{error}</p>
{/if}

<div class="mt-6 flex flex-col gap-6">
  {#each STATE_TYPE_ORDER as group (group.type)}
    {@const rows = groupFor(group.type)}
    <div class="rounded-lg border border-border bg-card">
      <div
        class="flex items-center justify-between border-b border-border px-4 py-2.5"
      >
        <div>
          <h4 class="text-sm font-medium text-foreground">{group.label}</h4>
          <p class="text-xs text-muted-foreground">{group.description}</p>
        </div>
        {#if canManage}
          <button
            aria-label="Add {group.label} state"
            class="flex h-7 items-center gap-1 rounded-md border border-border px-2 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
            onclick={() => startCreate(group.type)}
            type="button"
          >
            <Plus class="size-3.5" />
            Add
          </button>
        {/if}
      </div>

      <div class="divide-y divide-border">
        {#each rows as state, index (state._id)}
          <StateRow
            {canManage}
            isFirst={index === 0}
            issueState={state}
            isLast={index === rows.length - 1}
            onDelete={async () => {
              await removeState({ stateId: state._id });
            }}
            onMoveDown={() => swap(state, rows[index + 1])}
            onMoveUp={() => swap(state, rows[index - 1])}
            onSave={async (input) => {
              await updateState({ ...input, stateId: state._id });
            }}
            onSetDefault={async () => {
              await setDefaultState({ stateId: state._id });
            }}
          />
        {/each}

        {#if creatingType === group.type}
          <div class="flex flex-col gap-2 px-4 py-3">
            <div class="flex items-center gap-2">
              <input
                bind:value={newName}
                class="h-8 flex-1 rounded-md border border-border bg-background px-2 text-sm text-foreground focus:border-ring focus:outline-none"
                placeholder="State name"
                type="text"
              />
              <button
                class="h-8 rounded-md bg-primary px-2.5 text-xs font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
                disabled={!newName.trim()}
                onclick={submitCreate}
                type="button"
              >
                Create
              </button>
              <button
                class="h-8 rounded-md border border-border px-2.5 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
                onclick={() => (creatingType = null)}
                type="button"
              >
                Cancel
              </button>
            </div>
            <div class="flex flex-wrap items-center gap-1.5">
              {#each STATE_COLOR_PALETTE as swatch (swatch)}
                <button
                  aria-label="Use color {swatch}"
                  class="size-5 rounded-full border-2 {newColor === swatch
                    ? 'border-foreground'
                    : 'border-transparent'}"
                  onclick={() => (newColor = swatch)}
                  style="background-color:{swatch}"
                  type="button"
                ></button>
              {/each}
            </div>
          </div>
        {/if}

        {#if rows.length === 0 && creatingType !== group.type}
          <p class="px-4 py-3 text-xs text-muted-foreground">
            No states in this group.
          </p>
        {/if}
      </div>
    </div>
  {/each}
</div>
