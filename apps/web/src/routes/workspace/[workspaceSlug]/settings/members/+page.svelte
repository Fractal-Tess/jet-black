<script lang="ts">
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import WorkspaceMembersTab from "$lib/components/settings/workspace/WorkspaceMembersTab.svelte";

let { data } = $props();
const workspace = $derived(
  data.workspace as { _id: string; name: string; slug: string }
);
const membership = $derived(data.membership as { role: "owner" | "member" });
</script>

<SettingsContentWrapper>
  <WorkspaceMembersTab {workspace} role={membership.role} />
</SettingsContentWrapper>
