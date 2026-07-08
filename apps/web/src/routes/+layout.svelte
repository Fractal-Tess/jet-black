<script lang="ts">
import "../app.css";
let { children } = $props();
</script>

<svelte:head>
  <title>Jet Black</title>
  <meta name="description" content="A SvelteKit monorepo for apps, auth, workers, and scraping." />
</svelte:head>

{@render children()}
