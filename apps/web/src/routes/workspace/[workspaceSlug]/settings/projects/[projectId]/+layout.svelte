<script lang="ts">
import type { Snippet } from "svelte";
import SettingsNav from "$lib/components/settings/SettingsNav.svelte";
import SettingsShell from "$lib/components/settings/SettingsShell.svelte";
import { projectModuleHref } from "$lib/routes";
import { projectSettingsGroups, type WorkspaceRole } from "$lib/settings-nav";
import type { LayoutData } from "./$types";

let { data, children }: { data: LayoutData; children: Snippet } = $props();

const workspace = $derived(data.workspace as { name: string; slug: string });
const project = $derived(
  data.project as { _id: string; color: string; name: string }
);
const membership = $derived(data.membership as { role: WorkspaceRole });
const groups = $derived(projectSettingsGroups(workspace.slug, project._id));
</script>

<svelte:head>
  <title>Project settings · {project.name} · Jet Black</title>
</svelte:head>

<div class="flex h-screen flex-col bg-background pt-12">
  <SettingsShell>
    {#snippet sidebar()}
      <SettingsNav
        {groups}
        backHref={projectModuleHref({
          module: "tickets",
          projectId: project._id,
          workspaceSlug: workspace.slug,
        })}
        title="Project settings"
        entityName={project.name}
        entitySubtitle={membership.role}
        rootKey="general"
        role={membership.role}
      />
    {/snippet}

    {@render children()}
  </SettingsShell>
</div>
