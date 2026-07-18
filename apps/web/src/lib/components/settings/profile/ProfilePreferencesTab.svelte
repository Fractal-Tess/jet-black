<script lang="ts">
import SettingsControlRow from "$lib/components/settings/SettingsControlRow.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";

let theme = $state<"dark" | "light" | "system">("dark");
</script>

<SettingsHeading
  title="Preferences"
  description="Customize your Jet Black experience."
/>

<div class="mt-8 flex flex-col divide-y divide-border">
  <SettingsControlRow
    title="Theme"
    description="Choose your preferred color scheme."
  >
    {#snippet control()}
      <select
        bind:value={theme}
        class="h-9 rounded-md border border-border bg-card px-3 text-sm text-foreground focus:border-ring focus:outline-none"
      >
        <option value="dark">Dark</option>
        <option value="light">Light</option>
        <option value="system">System</option>
      </select>
    {/snippet}
  </SettingsControlRow>

  <SettingsControlRow
    title="Timezone"
    description="Used for displaying dates and times."
  >
    {#snippet control()}
      <select
        class="h-9 rounded-md border border-border bg-card px-3 text-sm text-foreground focus:border-ring focus:outline-none"
        value={Intl.DateTimeFormat().resolvedOptions().timeZone}
      >
        <option>{Intl.DateTimeFormat().resolvedOptions().timeZone}</option>
      </select>
    {/snippet}
  </SettingsControlRow>

  <SettingsControlRow
    title="First day of the week"
    description="Choose which day starts your calendar week."
  >
    {#snippet control()}
      <select
        class="h-9 rounded-md border border-border bg-card px-3 text-sm text-foreground focus:border-ring focus:outline-none"
      >
        <option value="0">Sunday</option>
        <option value="1" selected>Monday</option>
        <option value="6">Saturday</option>
      </select>
    {/snippet}
  </SettingsControlRow>
</div>
