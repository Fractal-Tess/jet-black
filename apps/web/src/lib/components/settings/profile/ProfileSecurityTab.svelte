<script lang="ts">
import Eye from "lucide-svelte/icons/eye";
import EyeOff from "lucide-svelte/icons/eye-off";
import { authClient } from "$lib/auth-client";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";

let currentPassword = $state("");
let newPassword = $state("");
let confirmPassword = $state("");
let showCurrent = $state(false);
let showNew = $state(false);
let showConfirm = $state(false);
let saving = $state(false);
let success = $state(false);
let error = $state("");

const passwordsMatch = $derived(newPassword === confirmPassword);
const newDiffersFromCurrent = $derived(newPassword !== currentPassword);
const canSubmit = $derived(
  currentPassword.length > 0 &&
    newPassword.length >= 8 &&
    passwordsMatch &&
    newDiffersFromCurrent &&
    !saving
);

async function handleSubmit() {
  saving = true;
  error = "";
  success = false;

  try {
    const result = await authClient.changePassword({
      currentPassword,
      newPassword,
    });

    if (result.error) {
      error = result.error.message ?? "Failed to change password.";
      return;
    }

    success = true;
    currentPassword = "";
    newPassword = "";
    confirmPassword = "";
    setTimeout(() => {
      success = false;
    }, 3000);
  } catch (e) {
    error = e instanceof Error ? e.message : "Failed to change password.";
  } finally {
    saving = false;
  }
}
</script>

<SettingsHeading
  title="Security"
  description="Manage your password and account security."
/>

<form
  class="mt-8 flex flex-col gap-6"
  onsubmit={(e) => {
    e.preventDefault();
    handleSubmit();
  }}
>
  <!-- Current password -->
  <div class="flex flex-col gap-1.5">
    <label class="text-sm font-medium text-foreground" for="current-password">
      Current password
    </label>
    <div class="relative max-w-sm">
      <input
        autocomplete="current-password"
        bind:value={currentPassword}
        class="h-9 w-full rounded-md border border-border bg-card px-3 pr-10 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
        id="current-password"
        placeholder="Enter current password"
        type={showCurrent ? "text" : "password"}
      />
      <button
        class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
        onclick={() => (showCurrent = !showCurrent)}
        tabindex={-1}
        type="button"
      >
        {#if showCurrent}
          <EyeOff class="size-4" />
        {:else}
          <Eye class="size-4" />
        {/if}
      </button>
    </div>
  </div>

  <!-- New passwords -->
  <div class="grid max-w-2xl grid-cols-1 gap-6 sm:grid-cols-2">
    <div class="flex flex-col gap-1.5">
      <label class="text-sm font-medium text-foreground" for="new-password">
        New password
      </label>
      <div class="relative">
        <input
          autocomplete="new-password"
          bind:value={newPassword}
          class="h-9 w-full rounded-md border border-border bg-card px-3 pr-10 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
          id="new-password"
          minlength={8}
          placeholder="At least 8 characters"
          type={showNew ? "text" : "password"}
        />
        <button
          class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          onclick={() => (showNew = !showNew)}
          tabindex={-1}
          type="button"
        >
          {#if showNew}
            <EyeOff class="size-4" />
          {:else}
            <Eye class="size-4" />
          {/if}
        </button>
      </div>
      {#if newPassword.length > 0 && !newDiffersFromCurrent}
        <p class="text-xs text-destructive">
          New password must differ from current password.
        </p>
      {/if}
    </div>

    <div class="flex flex-col gap-1.5">
      <label
        class="text-sm font-medium text-foreground"
        for="confirm-password"
      >
        Confirm new password
      </label>
      <div class="relative">
        <input
          autocomplete="new-password"
          bind:value={confirmPassword}
          class="h-9 w-full rounded-md border border-border bg-card px-3 pr-10 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
          id="confirm-password"
          placeholder="Confirm password"
          type={showConfirm ? "text" : "password"}
        />
        <button
          class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          onclick={() => (showConfirm = !showConfirm)}
          tabindex={-1}
          type="button"
        >
          {#if showConfirm}
            <EyeOff class="size-4" />
          {:else}
            <Eye class="size-4" />
          {/if}
        </button>
      </div>
      {#if confirmPassword.length > 0 && !passwordsMatch}
        <p class="text-xs text-destructive">Passwords do not match.</p>
      {/if}
    </div>
  </div>

  <!-- Submit -->
  <div class="flex items-center gap-3">
    <button
      class="h-9 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
      disabled={!canSubmit}
      type="submit"
    >
      {saving ? "Changing…" : "Change password"}
    </button>
    {#if success}
      <span class="text-xs text-primary">
        Password changed successfully.
      </span>
    {/if}
    {#if error}
      <span class="text-xs text-destructive" role="alert">{error}</span>
    {/if}
  </div>
</form>
