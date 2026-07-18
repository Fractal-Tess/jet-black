<script lang="ts">
import FileText from "lucide-svelte/icons/file-text";
import ProjectFeatureToggle from "$lib/components/settings/project/ProjectFeatureToggle.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
</script>

<SettingsContentWrapper>
  <ProjectFeatureToggle
    title="Pages"
    description="Create and share documentation within your project."
    icon={FileText}
  />
</SettingsContentWrapper>
