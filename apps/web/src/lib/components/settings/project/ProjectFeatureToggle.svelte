<script lang="ts">
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import type { SettingsNavIcon } from "$lib/settings-nav";

let {
  title,
  description,
  icon: Icon,
  enabled = true,
  onToggle,
}: {
  title: string;
  description: string;
  icon: SettingsNavIcon;
  enabled?: boolean;
  onToggle?: (value: boolean) => void;
} = $props();
</script>

<SettingsHeading {title} {description} />

<div class="mt-6">
  <SettingsBoxedControl
    title="{title} feature"
    description="Enable or disable this feature for the project. Disabling hides it from navigation without deleting data."
  >
    {#snippet control()}
      <label class="relative inline-flex cursor-pointer items-center">
        <input
          checked={enabled}
          class="peer sr-only"
          onchange={() => onToggle?.(!enabled)}
          type="checkbox"
        />
        <div
          class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"
        ></div>
      </label>
    {/snippet}
  </SettingsBoxedControl>
</div>
