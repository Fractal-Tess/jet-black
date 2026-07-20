<script lang="ts">
import WorkspaceApp from "$lib/components/workspace/WorkspaceApp.svelte";

let { data } = $props();
</script>

<svelte:head>
  <title>Module · Jet Black</title>
</svelte:head>

<WorkspaceApp
  {data}
  module={data.module}
  moduleId={data.moduleId}
  projectId={data.projectId}
  workspaceSlug={data.workspaceSlug}
/>
