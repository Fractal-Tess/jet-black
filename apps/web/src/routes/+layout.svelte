<script lang="ts">
import "../app.css";
let { children } = $props();
</script>

<svelte:head>
  <title>Jet Black</title>
  <meta
    name="description"
    content="A real-time workspace for projects, issues, and code review."
  />
</svelte:head>

{@render children()}
