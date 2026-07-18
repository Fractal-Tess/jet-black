<script lang="ts">
import type { Snippet } from "svelte";
import type { ProjectModule } from "$lib/routes";

let {
  activeModule,
  intake,
  modules,
  pages,
  placeholder,
  sprints,
  tickets,
}: {
  activeModule: ProjectModule;
  intake: Snippet;
  modules: Snippet;
  pages: Snippet;
  placeholder: Snippet<[{ module: ProjectModule }]>;
  sprints: Snippet;
  tickets: Snippet;
} = $props();
</script>

{#if activeModule === "intake"}
  {@render intake()}
{:else if activeModule === "sprints"}
  {@render sprints()}
{:else if activeModule === "modules"}
  {@render modules()}
{:else if activeModule === "pages"}
  {@render pages()}
{:else if activeModule === "views"}
  {@render placeholder({ module: "views" })}
{:else}
  {@render tickets()}
{/if}
