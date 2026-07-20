<script lang="ts">
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import Trash2 from "lucide-svelte/icons/trash-2";
import type { WorkspaceRole } from "$lib/settings-nav";

let {
  member,
  actorRole,
  isSelf,
  onChangeRole,
  onRemove,
}: {
  member: {
    id: string;
    name: string;
    email: string;
    image: string | null;
    role: WorkspaceRole;
  };
  actorRole: WorkspaceRole;
  isSelf: boolean;
  onChangeRole: (role: WorkspaceRole) => Promise<void>;
  onRemove: () => Promise<void>;
} = $props();

const ROLE_RANK: Record<WorkspaceRole, number> = {
  admin: 2,
  guest: 0,
  member: 1,
  owner: 3,
};

const canManage = $derived(
  !isSelf &&
    member.role !== "owner" &&
    ROLE_RANK[actorRole] > ROLE_RANK[member.role]
);

const assignableRoles = $derived(
  (["admin", "member", "guest"] as WorkspaceRole[]).filter(
    (role) => ROLE_RANK[actorRole] >= ROLE_RANK[role]
  )
);

let busy = $state(false);
let confirmingRemove = $state(false);

async function changeRole(role: WorkspaceRole) {
  busy = true;
  try {
    await onChangeRole(role);
  } finally {
    busy = false;
  }
}

async function remove() {
  busy = true;
  try {
    await onRemove();
  } finally {
    busy = false;
    confirmingRemove = false;
  }
}
</script>

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
      {#if isSelf}
        <span class="text-xs text-muted-foreground">(you)</span>
      {/if}
    </p>
    <p class="truncate text-xs text-muted-foreground">{member.email}</p>
  </div>

  {#if canManage}
    <Select
      onValueChange={(next) => {
        if (next && next !== member.role) {
          changeRole(next as WorkspaceRole);
        }
      }}
      type="single"
      value={member.role}
    >
      <SelectTrigger
        aria-label="Role for {member.name || member.email}"
        class="h-8 w-28 text-xs capitalize"
        disabled={busy}
      >
        {member.role}
      </SelectTrigger>
      <SelectContent>
        {#each assignableRoles as role (role)}
          <SelectItem class="text-sm capitalize" label={role} value={role} />
        {/each}
      </SelectContent>
    </Select>

    {#if confirmingRemove}
      <div class="flex items-center gap-1.5">
        <button
          class="h-8 rounded-md bg-destructive px-2.5 text-xs font-semibold text-destructive-foreground transition hover:opacity-90 disabled:opacity-50"
          disabled={busy}
          onclick={remove}
          type="button"
        >
          Remove
        </button>
        <button
          class="h-8 rounded-md border border-border px-2.5 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
          onclick={() => (confirmingRemove = false)}
          type="button"
        >
          Cancel
        </button>
      </div>
    {:else}
      <button
        aria-label="Remove {member.name || member.email}"
        class="grid size-8 place-items-center rounded-md text-muted-foreground transition hover:bg-destructive/10 hover:text-destructive"
        onclick={() => (confirmingRemove = true)}
        type="button"
      >
        <Trash2 class="size-3.5" />
      </button>
    {/if}
  {:else}
    <span
      class="rounded-md bg-muted px-2 py-0.5 text-xs font-medium capitalize text-muted-foreground"
    >
      {member.role}
    </span>
  {/if}
</div>
