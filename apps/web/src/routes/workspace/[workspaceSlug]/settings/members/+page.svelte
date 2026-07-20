<script lang="ts">
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import WorkspaceMembersTab from "$lib/components/settings/workspace/WorkspaceMembersTab.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

let { data } = $props();
const workspace = $derived(
  data.workspace as { _id: string; name: string; slug: string }
);
const membership = $derived(data.membership as { role: WorkspaceRole });
const currentUserId = $derived((data.user as { id: string }).id);
</script>

<SettingsContentWrapper>
  <WorkspaceMembersTab
    {currentUserId}
    role={membership.role}
    {workspace}
  />
</SettingsContentWrapper>
