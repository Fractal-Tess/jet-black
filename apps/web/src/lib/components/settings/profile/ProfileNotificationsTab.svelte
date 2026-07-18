<script lang="ts">
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";

let propertyChange = $state(true);
let stateChange = $state(true);
let issueCompleted = $state(true);
let comments = $state(true);
let mentions = $state(true);
</script>

<SettingsHeading
  title="Notifications"
  description="Manage your email notification preferences."
/>

<p class="mt-4 text-sm text-muted-foreground">
  Email notifications will be sent when these events occur. Delivery depends on
  email configuration.
</p>

<div class="mt-6 flex flex-col gap-3">
  <SettingsBoxedControl title="Property changes" description="When a work item's properties are updated.">
    {#snippet control()}
      <label class="relative inline-flex cursor-pointer items-center">
        <input bind:checked={propertyChange} class="peer sr-only" type="checkbox" />
        <div class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"></div>
      </label>
    {/snippet}
  </SettingsBoxedControl>

  <SettingsBoxedControl title="State changes" description="When a work item's state changes.">
    {#snippet control()}
      <label class="relative inline-flex cursor-pointer items-center">
        <input bind:checked={stateChange} class="peer sr-only" type="checkbox" />
        <div class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"></div>
      </label>
    {/snippet}
  </SettingsBoxedControl>

  <div class="ml-4 border-l-2 border-border pl-4">
    <SettingsBoxedControl title="Issue completed" description="When a work item is marked as completed.">
      {#snippet control()}
        <label class="relative inline-flex cursor-pointer items-center">
          <input bind:checked={issueCompleted} class="peer sr-only" disabled={!stateChange} type="checkbox" />
          <div class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary peer-disabled:opacity-50 after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"></div>
        </label>
      {/snippet}
    </SettingsBoxedControl>
  </div>

  <SettingsBoxedControl title="Comments" description="When someone comments on your work items.">
    {#snippet control()}
      <label class="relative inline-flex cursor-pointer items-center">
        <input bind:checked={comments} class="peer sr-only" type="checkbox" />
        <div class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"></div>
      </label>
    {/snippet}
  </SettingsBoxedControl>

  <SettingsBoxedControl title="Mentions" description="When someone mentions you in a comment or description.">
    {#snippet control()}
      <label class="relative inline-flex cursor-pointer items-center">
        <input bind:checked={mentions} class="peer sr-only" type="checkbox" />
        <div class="h-5 w-9 rounded-full bg-muted transition peer-checked:bg-primary after:absolute after:left-0.5 after:top-0.5 after:size-4 after:rounded-full after:bg-foreground after:transition-transform peer-checked:after:translate-x-4"></div>
      </label>
    {/snippet}
  </SettingsBoxedControl>
</div>
