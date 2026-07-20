<script lang="ts">
import type { SettingsNavIcon } from "$lib/settings-nav";

let {
  href,
  label,
  icon: Icon,
  active = false,
}: {
  href: string;
  label: string;
  icon: SettingsNavIcon;
  active?: boolean;
} = $props();
</script>

<a
  class="flex h-8 items-center gap-2 rounded-md px-2 text-sm font-medium transition-colors
    {active
    ? 'bg-sidebar-accent text-sidebar-accent-foreground'
    : 'text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'}"
  {href}
>
  <span class="grid size-4 shrink-0 place-items-center">
    <Icon
      class="size-3.5 {active
        ? 'text-sidebar-accent-foreground'
        : 'text-sidebar-muted-foreground'}"
    />
  </span>
  <span class="truncate">{label}</span>
</a>
