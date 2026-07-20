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
import TriangleAlert from "lucide-svelte/icons/triangle-alert";
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

type Automations = {
  autoArchiveClosedMonths: number | null;
  autoCloseInactiveMonths: number | null;
};

let {
  project,
  workspaceId,
  role,
}: {
  project: { _id: string; automations?: Automations };
  workspaceId: string;
  role: WorkspaceRole;
} = $props();

const MONTH_OPTIONS = [1, 3, 6, 9, 12];
const DEFAULT_MONTHS = 3;

const isAdmin = $derived(role === "owner" || role === "admin");
const auth = useAuth();
const projectId = $derived(project._id as Id<"projects">);

const projectQuery = useQuery(api.queries.workspaces.projectWithAccess, () =>
  auth.isAuthenticated
    ? { projectId, workspaceId: workspaceId as Id<"workspaces"> }
    : "skip"
);

const statesQuery = useQuery(api.queries.workspaces.statesForProject, () =>
  auth.isAuthenticated ? { projectId } : "skip"
);

const updateAutomations = useMutation(api.mutations.projects.updateAutomations);

const automations = $derived<Automations>(
  projectQuery.data?.automations ??
    project.automations ?? {
      autoArchiveClosedMonths: null,
      autoCloseInactiveMonths: null,
    }
);

const hasCancelledState = $derived(
  (statesQuery.data ?? []).some((state) => state.type === "cancelled")
);

let busy = $state(false);
let error = $state("");

async function persist(next: Automations) {
  busy = true;
  error = "";
  try {
    await updateAutomations({ automations: next, projectId });
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not save.";
  } finally {
    busy = false;
  }
}

function toggleArchive(enabled: boolean) {
  return persist({
    ...automations,
    autoArchiveClosedMonths: enabled ? DEFAULT_MONTHS : null,
  });
}

function setArchiveMonths(months: number) {
  return persist({ ...automations, autoArchiveClosedMonths: months });
}

function toggleClose(enabled: boolean) {
  return persist({
    ...automations,
    autoCloseInactiveMonths: enabled ? DEFAULT_MONTHS : null,
  });
}

function setCloseMonths(months: number) {
  return persist({ ...automations, autoCloseInactiveMonths: months });
}
</script>

<SettingsHeading
  title="Automations"
  description="Automatically tidy up stale issues in this project."
/>

<div class="mt-6 flex flex-col gap-4">
  <SettingsBoxedControl
    title="Auto-archive closed issues"
    description="Archive completed issues after a period of inactivity."
  >
    {#snippet control()}
      <div class="flex items-center gap-3">
        {#if automations.autoArchiveClosedMonths !== null}
          <Select
            onValueChange={(next) => {
              if (next) {
                setArchiveMonths(Number(next));
              }
            }}
            type="single"
            value={String(automations.autoArchiveClosedMonths)}
          >
            <SelectTrigger
              aria-label="Auto-archive after"
              class="h-8 w-32 text-xs"
              disabled={!isAdmin || busy}
            >
              {automations.autoArchiveClosedMonths} months
            </SelectTrigger>
            <SelectContent>
              {#each MONTH_OPTIONS as months (months)}
                <SelectItem
                  class="text-sm"
                  label="{months} months"
                  value={String(months)}
                />
              {/each}
            </SelectContent>
          </Select>
        {/if}
        <label class="relative inline-flex cursor-pointer items-center">
          <input
            checked={automations.autoArchiveClosedMonths !== null}
            class="peer sr-only"
            disabled={!isAdmin || busy}
            onchange={() =>
              toggleArchive(automations.autoArchiveClosedMonths === null)}
            type="checkbox"
          />
          <div
            class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"
          ></div>
        </label>
      </div>
    {/snippet}
  </SettingsBoxedControl>

  <SettingsBoxedControl
    title="Auto-close inactive issues"
    description="Cancel non-completed issues after a period of inactivity."
  >
    {#snippet control()}
      <div class="flex items-center gap-3">
        {#if automations.autoCloseInactiveMonths !== null}
          <Select
            onValueChange={(next) => {
              if (next) {
                setCloseMonths(Number(next));
              }
            }}
            type="single"
            value={String(automations.autoCloseInactiveMonths)}
          >
            <SelectTrigger
              aria-label="Auto-close after"
              class="h-8 w-32 text-xs"
              disabled={!isAdmin || busy}
            >
              {automations.autoCloseInactiveMonths} months
            </SelectTrigger>
            <SelectContent>
              {#each MONTH_OPTIONS as months (months)}
                <SelectItem
                  class="text-sm"
                  label="{months} months"
                  value={String(months)}
                />
              {/each}
            </SelectContent>
          </Select>
        {/if}
        <label class="relative inline-flex cursor-pointer items-center">
          <input
            checked={automations.autoCloseInactiveMonths !== null}
            class="peer sr-only"
            disabled={!isAdmin || busy}
            onchange={() =>
              toggleClose(automations.autoCloseInactiveMonths === null)}
            type="checkbox"
          />
          <div
            class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"
          ></div>
        </label>
      </div>
    {/snippet}
  </SettingsBoxedControl>

  {#if automations.autoCloseInactiveMonths !== null && !hasCancelledState}
    <div
      class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3"
    >
      <TriangleAlert class="mt-0.5 size-4 shrink-0 text-amber-500" />
      <p class="text-xs text-amber-600 dark:text-amber-400">
        This project has no cancelled state, so auto-close cannot run. Add a
        cancelled state in the States settings.
      </p>
    </div>
  {/if}

  {#if error}
    <p class="text-sm text-destructive" role="alert">{error}</p>
  {/if}
</div>
