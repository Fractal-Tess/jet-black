<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { useAuth, useMutation, useQuery } from "convex-svelte";
import Search from "lucide-svelte/icons/search";
import UserPlus from "lucide-svelte/icons/user-plus";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";
import InviteMemberDialog from "./InviteMemberDialog.svelte";
import MemberRow from "./MemberRow.svelte";

let {
  workspace,
  role,
  currentUserId,
}: {
  workspace: { _id: string; name: string; slug: string };
  role: WorkspaceRole;
  currentUserId: string;
} = $props();

const isAdmin = $derived(role === "owner" || role === "admin");
const auth = useAuth();
const workspaceId = $derived(workspace._id as Id<"workspaces">);

const membersQuery = useQuery(api.queries.workspaces.membersForWorkspace, () =>
  auth.isAuthenticated ? { workspaceId } : "skip"
);
const invitesQuery = useQuery(api.queries.workspaces.invitesForWorkspace, () =>
  auth.isAuthenticated && isAdmin ? { workspaceId } : "skip"
);

const inviteMember = useMutation(api.mutations.workspaceMembers.invite);
const revokeInvite = useMutation(api.mutations.workspaceMembers.revokeInvite);
const updateMemberRole = useMutation(api.mutations.workspaceMembers.updateRole);
const removeMember = useMutation(api.mutations.workspaceMembers.removeMember);

const members = $derived(membersQuery.data ?? []);
const invites = $derived(invitesQuery.data ?? []);
let search = $state("");
let inviteOpen = $state(false);
let actionError = $state("");

const filteredMembers = $derived(
  members.filter((m) => {
    if (!search.trim()) {
      return true;
    }
    const q = search.toLowerCase();
    return (
      m.name.toLowerCase().includes(q) || m.email.toLowerCase().includes(q)
    );
  })
);

async function runAction(action: () => Promise<unknown>) {
  actionError = "";
  try {
    await action();
  } catch (cause) {
    actionError =
      cause instanceof Error ? cause.message : "Something went wrong.";
  }
}
</script>

<SettingsHeading title="Members" description="Manage workspace members and roles.">
  {#snippet control()}
    <div class="flex items-center gap-2">
      <div
        class="flex h-9 items-center gap-2 rounded-md border border-border bg-card px-3"
      >
        <Search class="size-3.5 text-muted-foreground" />
        <input
          bind:value={search}
          class="w-48 bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
          placeholder="Search members…"
          type="text"
        />
      </div>
      <span
        class="flex h-6 items-center rounded-md bg-muted px-2 text-xs font-medium text-muted-foreground"
      >
        {members.length}
      </span>
      {#if isAdmin}
        <button
          class="flex h-9 items-center gap-1.5 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition hover:opacity-90"
          onclick={() => (inviteOpen = true)}
          type="button"
        >
          <UserPlus class="size-3.5" />
          Invite
        </button>
      {/if}
    </div>
  {/snippet}
</SettingsHeading>

{#if actionError}
  <p class="mt-4 text-sm text-destructive" role="alert">{actionError}</p>
{/if}

<div class="mt-6">
  {#if membersQuery.isLoading}
    <div class="py-8 text-center text-sm text-muted-foreground">
      Loading members…
    </div>
  {:else if filteredMembers.length === 0}
    <div class="py-8 text-center text-sm text-muted-foreground">
      {search ? "No members match your search." : "No members found."}
    </div>
  {:else}
    <div
      class="divide-y divide-border overflow-hidden rounded-lg border border-border"
    >
      {#each filteredMembers as member (member.id)}
        <MemberRow
          actorRole={role}
          isSelf={member.id === currentUserId}
          {member}
          onChangeRole={(nextRole) =>
            runAction(() =>
              updateMemberRole({
                memberUserId: member.id,
                role: nextRole,
                workspaceId,
              })
            )}
          onRemove={() =>
            runAction(() =>
              removeMember({ memberUserId: member.id, workspaceId })
            )}
        />
      {/each}
    </div>
  {/if}
</div>

{#if isAdmin && invites.length > 0}
  <div class="mt-8">
    <h4 class="mb-3 text-sm font-medium text-foreground">Pending invites</h4>
    <div
      class="divide-y divide-border overflow-hidden rounded-lg border border-border"
    >
      {#each invites as invite (invite._id)}
        <div class="flex items-center gap-4 px-4 py-3">
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-medium text-foreground">
              {invite.email}
            </p>
            <p class="truncate text-xs text-muted-foreground">
              Invited by {invite.invitedBy}
            </p>
          </div>
          <span
            class="rounded-md bg-muted px-2 py-0.5 text-xs font-medium capitalize text-muted-foreground"
          >
            {invite.role}
          </span>
          <button
            class="h-8 rounded-md border border-border px-2.5 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
            onclick={() =>
              runAction(() => revokeInvite({ inviteId: invite._id }))}
            type="button"
          >
            Revoke
          </button>
        </div>
      {/each}
    </div>
  </div>
{/if}

<InviteMemberDialog
  actorRole={role}
  isOpen={inviteOpen}
  onClose={() => (inviteOpen = false)}
  onInvite={async ({ email, role: inviteRole }) => {
    await inviteMember({ email, role: inviteRole, workspaceId });
  }}
/>
