<script lang="ts">
import Inbox from "lucide-svelte/icons/inbox";
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
    description="Collect and triage incoming work requests before they become work items."
    featureKey="intake"
    icon={Inbox}
    {project}
    role={membership.role}
    title="Intake"
    workspaceId={workspace._id}
  />
</SettingsContentWrapper>
