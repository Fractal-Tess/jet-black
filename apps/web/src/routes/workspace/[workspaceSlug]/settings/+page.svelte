<script lang="ts">
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import WorkspaceGeneralTab from "$lib/components/settings/workspace/WorkspaceGeneralTab.svelte";

let { data } = $props();
const workspace = $derived(
  data.workspace as {
    _id: string;
    name: string;
    slug: string;
    updatedAt: number;
  }
);
const membership = $derived(data.membership as { role: "owner" | "member" });
</script>

<SettingsContentWrapper>
  <WorkspaceGeneralTab {workspace} role={membership.role} />
</SettingsContentWrapper>
