<script lang="ts">
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";

let deleteOpen = $state(false);
let deleting = $state(false);
let deleteError = $state("");

async function handleDelete() {
  deleteError = "";
  deleting = true;

  try {
    const result = await authClient.deleteUser({
      callbackURL: "/login",
    });

    if (result.error) {
      deleteError = result.error.message ?? "Could not delete your account.";
      return;
    }

    deleteOpen = false;
    await goto("/login");
  } catch (error) {
    deleteError =
      error instanceof Error ? error.message : "Could not delete your account.";
  } finally {
    deleting = false;
  }
}
</script>

<SettingsHeading
  title="Danger zone"
  description="Irreversible and destructive actions for your account."
/>

<div class="mt-8">
  <SettingsBoxedControl
    danger
    title="Delete account"
    description="Permanently delete your account and all personal data. This action cannot be undone."
  >
    {#snippet control()}
      {#if deleteOpen}
        <div class="flex flex-col gap-2">
          <p class="text-xs text-destructive">
            Are you sure? This is permanent.
          </p>
          <div class="flex gap-2">
            <button
              class="h-8 rounded-md bg-destructive px-3 text-xs font-semibold text-destructive-foreground transition hover:opacity-90 disabled:opacity-50"
              disabled={deleting}
              onclick={handleDelete}
              type="button"
            >
              {deleting ? "Deleting…" : "Confirm delete account"}
            </button>
            <button
              class="h-8 rounded-md border border-border px-3 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground disabled:opacity-50"
              disabled={deleting}
              onclick={() => {
                deleteError = "";
                deleteOpen = false;
              }}
              type="button"
            >
              Cancel
            </button>
          </div>
          {#if deleteError}
            <p class="text-xs text-destructive" role="alert">{deleteError}</p>
          {/if}
        </div>
      {:else}
        <button
          class="h-8 rounded-md border border-destructive/50 px-3 text-xs font-medium text-destructive transition hover:bg-destructive/10"
          onclick={() => {
            deleteError = "";
            deleteOpen = true;
          }}
          type="button"
        >
          Delete account
        </button>
      {/if}
    {/snippet}
  </SettingsBoxedControl>
</div>
