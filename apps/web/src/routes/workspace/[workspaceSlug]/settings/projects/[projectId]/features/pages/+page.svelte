<script lang="ts">
import FileText from "lucide-svelte/icons/file-text";
import ProjectFeaturePage from "$lib/components/settings/project/ProjectFeaturePage.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import type { ProjectFeatures } from "$lib/project-features";
import type { WorkspaceRole } from "$lib/settings-nav";

let { data } = $props();
const project = $derived(
  data.project as { _id: string; features?: Partial<ProjectFeatures> }
);
const workspace = $derived(data.workspace as { _id: string });
const membership = $derived(data.membership as { role: WorkspaceRole });
</script>

<SettingsContentWrapper>
  <ProjectFeaturePage
    description="Create and share documentation within your project."
    featureKey="pages"
    icon={FileText}
    {project}
    role={membership.role}
    title="Pages"
    workspaceId={workspace._id}
  />
</SettingsContentWrapper>
