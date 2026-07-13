<script lang="ts">
import WorkspaceApp from "$lib/components/workspace/WorkspaceApp.svelte";

let { data } = $props();
</script>

<svelte:head>
  <title>Tickets · Jet Black</title>
</svelte:head>

<WorkspaceApp {data} />
