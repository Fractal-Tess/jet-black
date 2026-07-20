<script lang="ts">
import Menu from "lucide-svelte/icons/menu";
import X from "lucide-svelte/icons/x";
import type { Snippet } from "svelte";

let {
  sidebar,
  children,
  activePath = "",
}: {
  sidebar: Snippet;
  children: Snippet;
  activePath?: string;
} = $props();

let mobileOpen = $state(false);
</script>

<div class="flex h-full min-h-0 w-full">
  <!-- Desktop sidebar -->
  <div class="hidden h-full shrink-0 md:block">
    {@render sidebar()}
  </div>

  <!-- Mobile nav bar + dropdown -->
  <div class="contents md:hidden">
    <div
      class="fixed inset-x-0 top-12 z-40 flex h-11 items-center gap-3 border-b border-sidebar-border bg-background px-4"
    >
      <button
        aria-label={mobileOpen ? "Close settings menu" : "Open settings menu"}
        class="grid size-8 place-items-center rounded-md text-muted-foreground transition hover:bg-sidebar-accent hover:text-foreground"
        onclick={() => (mobileOpen = !mobileOpen)}
        type="button"
      >
        {#if mobileOpen}
          <X class="size-4" />
        {:else}
          <Menu class="size-4" />
        {/if}
      </button>
      {#if activePath}
      <span class="text-sm font-medium text-muted-foreground">
          {activePath}
        </span>
      {/if}
    </div>

    {#if mobileOpen}
      <button
        aria-label="Close settings menu"
        class="fixed inset-0 z-40 bg-background/50 md:hidden"
        onclick={() => (mobileOpen = false)}
        type="button"
      ></button>
      <div
        class="fixed left-0 top-[5.75rem] z-50 max-h-[calc(100vh-5.75rem)] overflow-y-auto rounded-br-lg border-b border-r border-sidebar-border bg-sidebar shadow-2xl"
      >
        <div onclick={() => (mobileOpen = false)} role="presentation">
          {@render sidebar()}
        </div>
      </div>
    {/if}
  </div>

  <!-- Content pane -->
  <div class="flex min-w-0 flex-1 flex-col overflow-y-auto pt-11 md:pt-0">
    {@render children()}
  </div>
</div>
