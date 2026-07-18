<script lang="ts">
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
</script>

<SettingsContentWrapper>
  <SettingsHeading
    title="Webhooks"
    description="Manage webhook endpoints for workspace events."
  />
  <div class="mt-8 rounded-lg border border-border bg-card px-6 py-10 text-center">
    <p class="text-sm text-muted-foreground">
      Webhook management is coming soon.
    </p>
  </div>
</SettingsContentWrapper>
