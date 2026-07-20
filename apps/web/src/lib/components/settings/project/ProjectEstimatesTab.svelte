<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import { useAuth, useMutation, useQuery } from "convex-svelte";
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import {
  ESTIMATE_PRESETS,
  type EstimatePreset,
  type EstimateSystem,
  presetValues,
} from "$lib/estimates";
import type { WorkspaceRole } from "$lib/settings-nav";

let {
  project,
  workspaceId,
  role,
}: {
  project: { _id: string; estimateSystem?: EstimateSystem };
  workspaceId: string;
  role: WorkspaceRole;
} = $props();

const CUSTOM_VALUE_SEPARATOR = /[\s,]+/;

const isAdmin = $derived(role === "owner" || role === "admin");
const auth = useAuth();
const projectId = $derived(project._id as Id<"projects">);

const projectQuery = useQuery(api.queries.workspaces.projectWithAccess, () =>
  auth.isAuthenticated
    ? { projectId, workspaceId: workspaceId as Id<"workspaces"> }
    : "skip"
);

const updateEstimateSystem = useMutation(
  api.mutations.projects.updateEstimateSystem
);

const estimateSystem = $derived(
  (projectQuery.data?.estimateSystem ??
    project.estimateSystem ??
    null) as EstimateSystem | null
);

let busy = $state(false);
let error = $state("");
let customInput = $state("");

$effect(() => {
  if (estimateSystem?.preset === "custom") {
    customInput = estimateSystem.values.join(", ");
  }
});

function presetLabel(preset: EstimatePreset): string {
  return ESTIMATE_PRESETS.find((entry) => entry.value === preset)?.label ?? "";
}

function parseCustomValues(raw: string): number[] {
  return raw
    .split(CUSTOM_VALUE_SEPARATOR)
    .map((part) => Number.parseInt(part, 10))
    .filter((value) => Number.isFinite(value) && value >= 0);
}

async function persist(next: EstimateSystem | null) {
  busy = true;
  error = "";
  try {
    await updateEstimateSystem({ estimateSystem: next, projectId });
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not save.";
  } finally {
    busy = false;
  }
}

async function toggle(enabled: boolean) {
  if (!isAdmin) {
    return;
  }

  if (!enabled) {
    await persist(null);
    return;
  }

  await persist({
    enabled: true,
    preset: "fibonacci",
    values: presetValues("fibonacci"),
  });
}

async function changePreset(preset: EstimatePreset) {
  if (!estimateSystem) {
    return;
  }

  if (preset === "custom") {
    await persist({ enabled: true, preset, values: estimateSystem.values });
    return;
  }

  await persist({ enabled: true, preset, values: presetValues(preset) });
}

async function saveCustom() {
  const values = parseCustomValues(customInput);
  if (values.length === 0) {
    error = "Enter at least one value";
    return;
  }

  await persist({ enabled: true, preset: "custom", values });
}
</script>

<SettingsHeading
  title="Estimates"
  description="Configure the estimation scale used for issues in this project."
/>

<div class="mt-6 flex flex-col gap-4">
  <SettingsBoxedControl
    title="Enable estimates"
    description="Turn on point-based estimates for issues in this project."
  >
    {#snippet control()}
      <label class="relative inline-flex cursor-pointer items-center">
        <input
          checked={Boolean(estimateSystem?.enabled)}
          class="peer sr-only"
          disabled={!isAdmin || busy}
          onchange={() => toggle(!estimateSystem?.enabled)}
          type="checkbox"
        />
        <div
          class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"
        ></div>
      </label>
    {/snippet}
  </SettingsBoxedControl>

  {#if estimateSystem?.enabled}
    <div class="rounded-lg border border-border bg-card p-4">
      <div class="flex items-center justify-between gap-4">
        <div>
          <h4 class="text-sm font-medium text-foreground">Scale</h4>
          <p class="text-xs text-muted-foreground">
            Choose a preset or define custom values.
          </p>
        </div>
        {#if isAdmin}
          <Select
            onValueChange={(next) => {
              if (next && next !== estimateSystem.preset) {
                changePreset(next as EstimatePreset);
              }
            }}
            type="single"
            value={estimateSystem.preset}
          >
            <SelectTrigger
              aria-label="Estimate preset"
              class="h-8 w-56 text-xs"
              disabled={busy}
            >
              {presetLabel(estimateSystem.preset)}
            </SelectTrigger>
            <SelectContent>
              {#each ESTIMATE_PRESETS as preset (preset.value)}
                <SelectItem
                  class="text-sm"
                  label={preset.label}
                  value={preset.value}
                />
              {/each}
            </SelectContent>
          </Select>
        {/if}
      </div>

      {#if estimateSystem.preset === "custom"}
        <div class="mt-4 flex items-center gap-2">
          <input
            bind:value={customInput}
            class="h-8 flex-1 rounded-md border border-border bg-background px-2 text-sm text-foreground focus:border-ring focus:outline-none"
            disabled={!isAdmin}
            placeholder="e.g. 1, 2, 4, 8"
            type="text"
          />
          {#if isAdmin}
            <button
              class="h-8 rounded-md bg-primary px-2.5 text-xs font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
              disabled={busy}
              onclick={saveCustom}
              type="button"
            >
              Save values
            </button>
          {/if}
        </div>
      {:else}
        <div class="mt-4 flex flex-wrap gap-1.5">
          {#each estimateSystem.values as value (value)}
            <span
              class="rounded-md border border-border bg-background px-2 py-0.5 text-xs text-foreground"
            >
              {value}
            </span>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if error}
    <p class="text-sm text-destructive" role="alert">{error}</p>
  {/if}
</div>
