<script lang="ts">
import { api } from "@workspace/convex/api";
import { useAuth, useQuery } from "convex-svelte";
import ProfileDangerTab from "$lib/components/settings/profile/ProfileDangerTab.svelte";
import ProfileGeneralTab from "$lib/components/settings/profile/ProfileGeneralTab.svelte";
import ProfileNotificationsTab from "$lib/components/settings/profile/ProfileNotificationsTab.svelte";
import ProfilePreferencesTab from "$lib/components/settings/profile/ProfilePreferencesTab.svelte";
import ProfileSecurityTab from "$lib/components/settings/profile/ProfileSecurityTab.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import SettingsNav from "$lib/components/settings/SettingsNav.svelte";
import SettingsShell from "$lib/components/settings/SettingsShell.svelte";
import { PROFILE_SETTINGS } from "$lib/settings-nav";

let { data } = $props();
const tab = $derived(data.tab);
const auth = useAuth();
const viewer = useQuery(api.queries.workspaces.viewer, () =>
  auth.isAuthenticated ? {} : "skip"
);
const user = $derived(
  viewer.data?.user ?? {
    email: data.user.email,
    id: data.user.id,
    image: data.user.image ?? null,
    name: data.user.name ?? "",
  }
);
</script>

<svelte:head>
  <title>Profile settings · Jet Black</title>
</svelte:head>

<div class="flex h-screen flex-col bg-background pt-12">
  <SettingsShell activePath={tab}>
    {#snippet sidebar()}
      <SettingsNav
        groups={PROFILE_SETTINGS}
        backHref="/dashboard"
        title="Profile settings"
        entityName={user.name || "User"}
        entitySubtitle={user.email}
      />
    {/snippet}

    <SettingsContentWrapper>
      {#if tab === "general"}
        <ProfileGeneralTab {user} />
      {:else if tab === "preferences"}
        <ProfilePreferencesTab />
      {:else if tab === "notifications"}
        <ProfileNotificationsTab />
      {:else if tab === "security"}
        <ProfileSecurityTab />
      {:else if tab === "danger"}
        <ProfileDangerTab />
      {:else}
        <div class="py-20 text-center text-muted-foreground">
          Unknown settings tab.
        </div>
      {/if}
    </SettingsContentWrapper>
  </SettingsShell>
</div>
