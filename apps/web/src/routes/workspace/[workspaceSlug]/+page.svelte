<script lang="ts">
import WorkspaceApp from "$lib/components/workspace/WorkspaceApp.svelte";

let { data } = $props();
</script>

<svelte:head>
  <title>Home · Jet Black</title>
</svelte:head>

<WorkspaceApp {data} workspaceSlug={data.workspaceSlug} />
