<script lang="ts">
import { page } from "$app/state";
import {
  filterGroupsByRole,
  isSettingsItemActive,
  type SettingsNavGroup,
  type WorkspaceRole,
} from "$lib/settings-nav";
import SettingsSidebar from "./SettingsSidebar.svelte";
import SettingsSidebarGroup from "./SettingsSidebarGroup.svelte";
import SettingsSidebarHeader from "./SettingsSidebarHeader.svelte";
import SettingsSidebarItem from "./SettingsSidebarItem.svelte";

let {
  groups,
  backHref,
  title,
  entityName,
  entitySubtitle,
  entityInitials,
  role,
  rootKey,
}: {
  groups: SettingsNavGroup[];
  backHref: string;
  title: string;
  entityName: string;
  entitySubtitle?: string;
  entityInitials?: string;
  role?: WorkspaceRole;
  rootKey?: string;
} = $props();

const filtered = $derived(filterGroupsByRole(groups, role));
const currentPath = $derived(page.url.pathname);
</script>

<SettingsSidebar>
  {#snippet header()}
    <SettingsSidebarHeader
      {backHref}
      {title}
      {entityName}
      {entitySubtitle}
      {entityInitials}
    />
  {/snippet}

  {#each filtered as group (group.category)}
    <SettingsSidebarGroup label={group.category}>
      {#each group.items as item (item.key)}
        <SettingsSidebarItem
          href={item.href}
          label={item.label}
          icon={item.icon}
          active={isSettingsItemActive(
            item.href,
            currentPath,
            item.key === rootKey
          )}
        />
      {/each}
    </SettingsSidebarGroup>
  {/each}
</SettingsSidebar>
