<script lang="ts">
import WorkspaceApp from "$lib/components/workspace/WorkspaceApp.svelte";

let { data } = $props();
</script>

<svelte:head>
  <title>Analytics · Jet Black</title>
</svelte:head>

<WorkspaceApp {data} workspacePage="analytics" workspaceSlug={data.workspaceSlug} />
