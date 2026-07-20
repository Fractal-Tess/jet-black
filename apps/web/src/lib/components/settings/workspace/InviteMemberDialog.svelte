<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@workspace/ui/components/dialog";
import { Input } from "@workspace/ui/components/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import type { WorkspaceRole } from "$lib/settings-nav";

let {
  isOpen = false,
  actorRole,
  onClose,
  onInvite,
}: {
  isOpen?: boolean;
  actorRole: WorkspaceRole;
  onClose: () => void;
  onInvite: (input: { email: string; role: WorkspaceRole }) => Promise<void>;
} = $props();

const ROLE_OPTIONS: { label: string; value: WorkspaceRole }[] = [
  { label: "Admin", value: "admin" },
  { label: "Member", value: "member" },
  { label: "Guest", value: "guest" },
];

const availableRoles = $derived(
  actorRole === "owner"
    ? ROLE_OPTIONS
    : ROLE_OPTIONS.filter((option) => option.value !== "admin")
);

let email = $state("");
let role = $state<WorkspaceRole>("member");
let sending = $state(false);
let error = $state("");

$effect(() => {
  if (isOpen) {
    email = "";
    role = "member";
    error = "";
  }
});

async function submit() {
  const trimmed = email.trim().toLowerCase();

  if (!trimmed.includes("@")) {
    error = "A valid email address is required";
    return;
  }

  sending = true;
  error = "";

  try {
    await onInvite({ email: trimmed, role });
    onClose();
  } catch (cause) {
    error =
      cause instanceof Error ? cause.message : "Could not send the invite.";
  } finally {
    sending = false;
  }
}
</script>

<Dialog
  onOpenChange={(open) => {
    if (!open) {
      onClose();
    }
  }}
  open={isOpen}
>
  <DialogContent class="max-w-md">
    <DialogHeader>
      <DialogTitle>Invite a member</DialogTitle>
    </DialogHeader>

    <form
      class="flex flex-col gap-3"
      onsubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-medium text-foreground" for="invite-email">
          Email
        </label>
        <Input
          bind:value={email}
          id="invite-email"
          placeholder="name@company.com"
          type="email"
        />
      </div>

      <div class="flex flex-col gap-1.5">
        <span class="text-sm font-medium text-foreground">Role</span>
        <Select
          onValueChange={(next) => {
            if (next) {
              role = next as WorkspaceRole;
            }
          }}
          type="single"
          value={role}
        >
          <SelectTrigger aria-label="Role" class="w-full capitalize">
            {role}
          </SelectTrigger>
          <SelectContent>
            {#each availableRoles as option (option.value)}
              <SelectItem
                class="text-sm"
                label={option.label}
                value={option.value}
              />
            {/each}
          </SelectContent>
        </Select>
        <p class="text-xs text-muted-foreground">
          The invitee joins automatically the next time they sign in with this
          email.
        </p>
      </div>

      {#if error}
        <p class="text-sm text-destructive" role="alert">{error}</p>
      {/if}

      <DialogFooter>
        <Button onclick={onClose} size="sm" variant="outline">Cancel</Button>
        <Button disabled={sending || !email.trim()} size="sm" type="submit">
          {sending ? "Inviting…" : "Send invite"}
        </Button>
      </DialogFooter>
    </form>
  </DialogContent>
</Dialog>
