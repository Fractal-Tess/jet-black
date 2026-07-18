<script lang="ts">
import Inbox from "lucide-svelte/icons/inbox";
import ProjectFeatureToggle from "$lib/components/settings/project/ProjectFeatureToggle.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
</script>

<SettingsContentWrapper>
  <ProjectFeatureToggle
    title="Intake"
    description="Collect and triage incoming work requests before they become work items."
    icon={Inbox}
  />
</SettingsContentWrapper>
