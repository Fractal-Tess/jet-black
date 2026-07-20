<script lang="ts">
import Moon from "lucide-svelte/icons/moon";
import Sun from "lucide-svelte/icons/sun";
import { toggleMode } from "mode-watcher";
</script>

<button
  aria-label="Toggle theme"
  class="relative grid size-8 cursor-pointer place-items-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
  onclick={toggleMode}
  type="button"
>
  <Sun
    class="size-4 scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90"
  />
  <Moon
    class="absolute size-4 scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0"
  />
</button>
