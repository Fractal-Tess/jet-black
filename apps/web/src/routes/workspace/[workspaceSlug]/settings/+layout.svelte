<script lang="ts">
import type { Snippet } from "svelte";
import SettingsNav from "$lib/components/settings/SettingsNav.svelte";
import SettingsShell from "$lib/components/settings/SettingsShell.svelte";
import { workspaceHref } from "$lib/routes";
import { workspaceSettingsGroups } from "$lib/settings-nav";
import type { LayoutData } from "./$types";

let { data, children }: { data: LayoutData; children: Snippet } = $props();

const workspace = $derived(data.workspace as { name: string; slug: string });
const membership = $derived(data.membership as { role: "owner" | "member" });
const slug = $derived(workspace.slug);
const groups = $derived(workspaceSettingsGroups(slug));
</script>

<svelte:head>
  <title>Workspace settings · {workspace.name} · Jet Black</title>
</svelte:head>

<div class="flex h-screen flex-col bg-background pt-12">
  <SettingsShell>
    {#snippet sidebar()}
      <SettingsNav
        {groups}
        backHref={workspaceHref(slug)}
        title="Workspace settings"
        entityName={workspace.name}
        entitySubtitle={membership.role}
        rootKey="general"
        role={membership.role}
      />
    {/snippet}

    {@render children()}
  </SettingsShell>
</div>
