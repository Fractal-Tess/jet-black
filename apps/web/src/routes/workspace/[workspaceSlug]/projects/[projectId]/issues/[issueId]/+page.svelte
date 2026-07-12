<script lang="ts">
import WorkspaceApp from "$lib/components/workspace/WorkspaceApp.svelte";

let { data } = $props();
</script>

<svelte:head>
  <title>Issue · Jet Black</title>
</svelte:head>

<WorkspaceApp
  {data}
  module={data.module}
  projectId={data.projectId}
  routeIssueId={data.issueId}
  workspaceSlug={data.workspaceSlug}
/>
