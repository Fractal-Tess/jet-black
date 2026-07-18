import AlertTriangle from "lucide-svelte/icons/alert-triangle";
import ArrowUpToLine from "lucide-svelte/icons/arrow-up-to-line";
import Bell from "lucide-svelte/icons/bell";
import Building from "lucide-svelte/icons/building";
import CircleDot from "lucide-svelte/icons/circle-dot";
import CircleUser from "lucide-svelte/icons/circle-user";
import FileText from "lucide-svelte/icons/file-text";
import Gauge from "lucide-svelte/icons/gauge";
import Inbox from "lucide-svelte/icons/inbox";
import Layers from "lucide-svelte/icons/layers";
import LayoutGrid from "lucide-svelte/icons/layout-grid";
import Lock from "lucide-svelte/icons/lock";
import Settings from "lucide-svelte/icons/settings";
import Settings2 from "lucide-svelte/icons/settings-2";
import Tag from "lucide-svelte/icons/tag";
import Timer from "lucide-svelte/icons/timer";
import Users from "lucide-svelte/icons/users";
import Webhook from "lucide-svelte/icons/webhook";
import Zap from "lucide-svelte/icons/zap";

export type WorkspaceRole = "owner" | "admin" | "member" | "guest";

export type SettingsNavIcon = typeof Settings;

export type SettingsNavItem = {
  key: string;
  label: string;
  icon: SettingsNavIcon;
  href: string;
  access?: WorkspaceRole[];
};

export type SettingsNavGroup = {
  category: string;
  items: SettingsNavItem[];
};

// ---------------------------------------------------------------------------
// Profile settings
// ---------------------------------------------------------------------------

export const PROFILE_SETTINGS: SettingsNavGroup[] = [
  {
    category: "Your profile",
    items: [
      {
        key: "general",
        label: "Profile",
        icon: CircleUser,
        href: "/settings/profile/general",
      },
      {
        key: "preferences",
        label: "Preferences",
        icon: Settings2,
        href: "/settings/profile/preferences",
      },
      {
        key: "notifications",
        label: "Notifications",
        icon: Bell,
        href: "/settings/profile/notifications",
      },
      {
        key: "security",
        label: "Security",
        icon: Lock,
        href: "/settings/profile/security",
      },
    ],
  },
  {
    category: "Account",
    items: [
      {
        key: "danger",
        label: "Danger zone",
        icon: AlertTriangle,
        href: "/settings/profile/danger",
      },
    ],
  },
];

// ---------------------------------------------------------------------------
// Workspace settings
// ---------------------------------------------------------------------------

export function workspaceSettingsGroups(slug: string): SettingsNavGroup[] {
  const base = `/workspace/${slug}/settings`;
  return [
    {
      category: "Administration",
      items: [
        {
          key: "general",
          label: "General",
          icon: Building,
          href: base,
          access: ["owner", "admin", "member"],
        },
        {
          key: "members",
          label: "Members",
          icon: Users,
          href: `${base}/members`,
          access: ["owner", "admin", "member"],
        },
        {
          key: "export",
          label: "Export",
          icon: ArrowUpToLine,
          href: `${base}/exports`,
          access: ["owner", "admin", "member"],
        },
      ],
    },
    {
      category: "Developer",
      items: [
        {
          key: "webhooks",
          label: "Webhooks",
          icon: Webhook,
          href: `${base}/webhooks`,
          access: ["owner", "admin"],
        },
      ],
    },
  ];
}

// ---------------------------------------------------------------------------
// Project settings
// ---------------------------------------------------------------------------

export function projectSettingsGroups(
  slug: string,
  projectId: string
): SettingsNavGroup[] {
  const base = `/workspace/${slug}/settings/projects/${projectId}`;
  return [
    {
      category: "General",
      items: [
        {
          key: "general",
          label: "General",
          icon: Settings,
          href: base,
          access: ["owner", "admin", "member", "guest"],
        },
        {
          key: "members",
          label: "Members",
          icon: Users,
          href: `${base}/members`,
          access: ["owner", "admin", "member", "guest"],
        },
      ],
    },
    {
      category: "Features",
      items: [
        {
          key: "sprints",
          label: "Sprints",
          icon: Timer,
          href: `${base}/features/cycles`,
          access: ["owner", "admin"],
        },
        {
          key: "modules",
          label: "Modules",
          icon: Layers,
          href: `${base}/features/modules`,
          access: ["owner", "admin"],
        },
        {
          key: "views",
          label: "Views",
          icon: LayoutGrid,
          href: `${base}/features/views`,
          access: ["owner", "admin"],
        },
        {
          key: "pages",
          label: "Pages",
          icon: FileText,
          href: `${base}/features/pages`,
          access: ["owner", "admin"],
        },
        {
          key: "intake",
          label: "Intake",
          icon: Inbox,
          href: `${base}/features/intake`,
          access: ["owner", "admin"],
        },
      ],
    },
    {
      category: "Work structure",
      items: [
        {
          key: "states",
          label: "States",
          icon: CircleDot,
          href: `${base}/states`,
          access: ["owner", "admin", "member"],
        },
        {
          key: "labels",
          label: "Labels",
          icon: Tag,
          href: `${base}/labels`,
          access: ["owner", "admin", "member"],
        },
        {
          key: "estimates",
          label: "Estimates",
          icon: Gauge,
          href: `${base}/estimates`,
          access: ["owner", "admin"],
        },
      ],
    },
    {
      category: "Execution",
      items: [
        {
          key: "automations",
          label: "Automations",
          icon: Zap,
          href: `${base}/automations`,
          access: ["owner", "admin"],
        },
      ],
    },
  ];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TRAILING_SLASH = /\/$/;

export function filterGroupsByRole(
  groups: SettingsNavGroup[],
  role?: WorkspaceRole
): SettingsNavGroup[] {
  return groups
    .map((group) => ({
      ...group,
      items: group.items.filter(
        (item) => !(item.access && role) || item.access.includes(role)
      ),
    }))
    .filter((group) => group.items.length > 0);
}

export function isSettingsItemActive(
  itemHref: string,
  currentPath: string,
  isRoot = false
): boolean {
  const normalized = currentPath.replace(TRAILING_SLASH, "");
  const target = itemHref.replace(TRAILING_SLASH, "");

  if (isRoot || target === normalized) {
    return normalized === target;
  }

  return normalized.startsWith(target);
}
