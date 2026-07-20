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
import Trash2 from "lucide-svelte/icons/trash-2";
import UserPlus from "lucide-svelte/icons/user-plus";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

type ProjectRole = "admin" | "member";

let {
  project,
  workspaceId,
  role,
}: {
  project: { _id: string; name: string };
  workspaceId: string;
  role: WorkspaceRole;
} = $props();

const isAdmin = $derived(role === "owner" || role === "admin");
const auth = useAuth();
const projectId = $derived(project._id as Id<"projects">);

const rosterQuery = useQuery(api.queries.projectMembers.listForProject, () =>
  auth.isAuthenticated ? { projectId } : "skip"
);
const workspaceMembersQuery = useQuery(
  api.queries.workspaces.membersForWorkspace,
  () =>
    auth.isAuthenticated && isAdmin
      ? { workspaceId: workspaceId as Id<"workspaces"> }
      : "skip"
);

const addMember = useMutation(api.mutations.projectMembers.add);
const updateRole = useMutation(api.mutations.projectMembers.updateRole);
const removeMember = useMutation(api.mutations.projectMembers.remove);

const roster = $derived(rosterQuery.data ?? []);
const rosterIds = $derived(new Set(roster.map((member) => member.id)));
const addableMembers = $derived(
  (workspaceMembersQuery.data ?? []).filter(
    (member) => !rosterIds.has(member.id)
  )
);

let addSelection = $state<string>("");
let error = $state("");

async function run(action: () => Promise<unknown>) {
  error = "";
  try {
    await action();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Something went wrong.";
  }
}
</script>

<SettingsHeading
  title="Members"
  description="Manage which workspace members belong to this project."
>
  {#snippet control()}
    <span
      class="flex h-6 items-center rounded-md bg-muted px-2 text-xs font-medium text-muted-foreground"
    >
      {roster.length}
    </span>
  {/snippet}
</SettingsHeading>

{#if error}
  <p class="mt-4 text-sm text-destructive" role="alert">{error}</p>
{/if}

{#if isAdmin}
  <div class="mt-6 flex items-center gap-2">
    <Select
      onValueChange={(next) => (addSelection = next ?? "")}
      type="single"
      value={addSelection}
    >
      <SelectTrigger aria-label="Add member" class="h-9 w-64 text-sm">
        {#if addSelection}
          {addableMembers.find((m) => m.id === addSelection)?.name ??
            "Select a member"}
        {:else}
          Add a member…
        {/if}
      </SelectTrigger>
      <SelectContent>
        {#each addableMembers as member (member.id)}
          <SelectItem
            class="text-sm"
            label={member.name || member.email}
            value={member.id}
          />
        {/each}
      </SelectContent>
    </Select>
    <button
      class="flex h-9 items-center gap-1.5 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
      disabled={!addSelection}
      onclick={() =>
        run(async () => {
          await addMember({ projectId, role: "member", userId: addSelection });
          addSelection = "";
        })}
      type="button"
    >
      <UserPlus class="size-3.5" />
      Add
    </button>
  </div>
{/if}

<div class="mt-6">
  {#if rosterQuery.isLoading}
    <div class="py-8 text-center text-sm text-muted-foreground">
      Loading members…
    </div>
  {:else if roster.length === 0}
    <div class="py-8 text-center text-sm text-muted-foreground">
      No project members yet.
    </div>
  {:else}
    <div
      class="divide-y divide-border overflow-hidden rounded-lg border border-border"
    >
      {#each roster as member (member.id)}
        <div class="flex items-center gap-4 px-4 py-3">
          <div
            class="grid size-8 shrink-0 place-items-center rounded-full bg-primary text-meta font-bold text-primary-foreground"
          >
            {member.name
              ?.trim()
              .split(/\s+/)
              .slice(0, 2)
              .map((w) => w[0])
              .join("")
              .toUpperCase() ?? member.email.slice(0, 2).toUpperCase()}
          </div>
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-medium text-foreground">
              {member.name || "Unnamed"}
            </p>
            <p class="truncate text-xs text-muted-foreground">{member.email}</p>
          </div>

          {#if isAdmin}
            <Select
              onValueChange={(next) => {
                if (next && next !== member.role) {
                  run(() =>
                    updateRole({
                      projectId,
                      role: next as ProjectRole,
                      userId: member.id,
                    })
                  );
                }
              }}
              type="single"
              value={member.role}
            >
              <SelectTrigger
                aria-label="Role for {member.name || member.email}"
                class="h-8 w-28 text-xs capitalize"
              >
                {member.role}
              </SelectTrigger>
              <SelectContent>
                <SelectItem class="text-sm" label="Admin" value="admin" />
                <SelectItem class="text-sm" label="Member" value="member" />
              </SelectContent>
            </Select>
            <button
              aria-label="Remove {member.name || member.email}"
              class="grid size-8 place-items-center rounded-md text-muted-foreground transition hover:bg-destructive/10 hover:text-destructive"
              onclick={() =>
                run(() => removeMember({ projectId, userId: member.id }))}
              type="button"
            >
              <Trash2 class="size-3.5" />
            </button>
          {:else}
            <span
              class="rounded-md bg-muted px-2 py-0.5 text-xs font-medium capitalize text-muted-foreground"
            >
              {member.role}
            </span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
