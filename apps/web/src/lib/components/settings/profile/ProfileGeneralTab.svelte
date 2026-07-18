<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { useMutation } from "convex-svelte";
import { untrack } from "svelte";
import { authClient } from "$lib/auth-client";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import AvatarEditor from "./AvatarEditor.svelte";

let {
  user,
}: {
  user: { email: string; id: string; image: string | null; name: string };
} = $props();

let name = $state(untrack(() => user.name));
let saving = $state(false);
let saveSuccess = $state(false);
let saveError = $state("");

const generateUploadUrl = useMutation(api.mutations.users.generateUploadUrl);
const getStorageUrl = useMutation(api.mutations.users.getStorageUrl);

const initials = $derived(
  user.name
    ?.trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => w[0])
    .join("")
    .toUpperCase() ?? user.email.slice(0, 2).toUpperCase()
);

async function handleSave() {
  saving = true;
  saveError = "";
  saveSuccess = false;

  try {
    const result = await authClient.updateUser({ name });

    if (result.error) {
      saveError = result.error.message ?? "Failed to update profile.";
      return;
    }

    saveSuccess = true;
    setTimeout(() => {
      saveSuccess = false;
    }, 2000);
  } catch (error) {
    saveError =
      error instanceof Error ? error.message : "Failed to update profile.";
  } finally {
    saving = false;
  }
}

async function handleAvatarSave(file: File) {
  const uploadUrl = await generateUploadUrl({});
  const uploadResponse = await fetch(uploadUrl, {
    method: "POST",
    headers: { "Content-Type": file.type },
    body: file,
  });

  if (!uploadResponse.ok) {
    throw new Error("Upload failed");
  }

  const { storageId } = (await uploadResponse.json()) as {
    storageId: Id<"_storage">;
  };
  const imageUrl = await getStorageUrl({ storageId });

  if (!imageUrl) {
    throw new Error("Failed to get image URL");
  }

  await authClient.updateUser({ image: imageUrl });
}

$effect(() => {
  name = user.name;
});
</script>

<SettingsHeading
  title="Profile"
  description="Manage your personal information."
/>

<div class="mt-8 flex flex-col gap-8">
  <!-- Avatar editor -->
  <AvatarEditor
    currentImage={user.image}
    {initials}
    onSave={handleAvatarSave}
  />

  <!-- Form fields -->
  <form
    class="flex flex-col gap-6"
    onsubmit={(e) => {
      e.preventDefault();
      handleSave();
    }}
  >
    <div class="grid grid-cols-1 gap-6 sm:grid-cols-2">
      <!-- Name -->
      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-medium text-foreground" for="profile-name">
          Display name
        </label>
        <input
          bind:value={name}
          class="h-9 w-full rounded-md border border-border bg-card px-3 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
          id="profile-name"
          placeholder="Your name"
          type="text"
        />
      </div>

      <!-- Email (read-only) -->
      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-medium text-foreground" for="profile-email">
          Email
        </label>
        <input
          class="h-9 w-full cursor-not-allowed rounded-md border border-border bg-muted px-3 text-sm text-muted-foreground"
          disabled
          id="profile-email"
          type="email"
          value={user.email}
        />
        <p class="text-xs text-muted-foreground">
          Email changes are not yet supported.
        </p>
      </div>
    </div>

    <!-- Save -->
    <div class="flex items-center gap-3">
      <button
        class="h-9 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
        disabled={saving || name === user.name}
        type="submit"
      >
        {saving ? "Saving…" : "Update profile"}
      </button>
      {#if saveSuccess}
        <span class="text-xs text-primary">Saved successfully.</span>
      {/if}
      {#if saveError}
        <span class="text-xs text-destructive" role="alert">{saveError}</span>
      {/if}
    </div>
  </form>
</div>
