<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { useAuth, useQuery } from "convex-svelte";
import Search from "lucide-svelte/icons/search";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";

let {
  workspace,
  role,
}: {
  workspace: { _id: string; name: string; slug: string };
  role: "owner" | "member";
} = $props();

const isOwner = $derived(role === "owner");
const auth = useAuth();
const membersQuery = useQuery(api.queries.workspaces.membersForWorkspace, () =>
  auth.isAuthenticated
    ? { workspaceId: workspace._id as Id<"workspaces"> }
    : "skip"
);

const members = $derived(membersQuery.data ?? []);
let search = $state("");
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
    </div>
  {/snippet}
</SettingsHeading>

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
    <div class="divide-y divide-border overflow-hidden rounded-lg border border-border">
      {#each filteredMembers as member (member.id)}
        <div class="flex items-center gap-4 px-4 py-3">
          <!-- Avatar -->
          <div
            class="grid size-8 shrink-0 place-items-center rounded-full bg-primary text-[11px] font-bold text-primary-foreground"
          >
            {member.name
              ?.trim()
              .split(/\s+/)
              .slice(0, 2)
              .map((w) => w[0])
              .join("")
              .toUpperCase() ?? member.email.slice(0, 2).toUpperCase()}
          </div>

          <!-- Name + email -->
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-medium text-foreground">
              {member.name || "Unnamed"}
            </p>
            <p class="truncate text-xs text-muted-foreground">
              {member.email}
            </p>
          </div>

          <!-- Role -->
          <span
            class="rounded-md bg-muted px-2 py-0.5 text-xs font-medium capitalize text-muted-foreground"
          >
            {member.role}
          </span>
        </div>
      {/each}
    </div>
  {/if}
</div>
