# Jet Black Plane Settings Alignment TODO

> Product-behavior reference only. ADR-007 supersedes this document's
> Convex-only and Better Auth implementation constraints. Implement remaining
> settings behavior through the Rust control plane and typed client.

## Goal

Align Jet Black's profile, workspace, project-settings, and settings-entry navigation as closely as practical with the product behavior in the local `references/plane` source.

Plane is the product and interaction reference. Jet Black keeps its SvelteKit + Convex + Better Auth architecture, `/workspace/[workspaceSlug]` route prefix, current design tokens, and realtime Convex subscriptions. Recreate Plane's information architecture, visual hierarchy, permission behavior, and interaction patterns without copying its React implementation.

This file is the focused execution plan for settings parity. Unrelated issue-detail, module-assignment, and general shell work remains in `TODO.md`.

## Non-negotiable constraints

- [ ] Use the local `references/plane` checkout as the source of truth for settings scope and behavior.
- [ ] Use Convex as the only backend, data, authorization, storage, scheduling, and function layer.
- [ ] Keep Better Auth authoritative for identity and credentials: email, name, image, password, sessions, and account deletion.
- [ ] Do not duplicate Convex query, mutation, or document shapes in frontend code. Derive types with `FunctionReturnType`, `FunctionArgs`, `Doc`, and `Id`.
- [ ] Do not introduce explicit `any`, `as any`, `@ts-ignore`, or `@ts-expect-error` in handwritten source.
- [ ] Keep new Svelte and TypeScript files focused. Target fewer than 250 logical lines per file and split by domain before components become oversized.
- [ ] Validate every public Convex function's arguments and authorize the authenticated user at the correct workspace or project scope.
- [ ] Treat frontend permission filtering as presentation only. Convex must enforce every protected read, write, and destructive operation.
- [ ] Require confirmation for destructive operations and enforce their invariants in Convex.
- [ ] Preserve realtime behavior with Convex subscriptions.
- [ ] Use existing Jet Black tokens and shared UI primitives while matching Plane's density, layout, and control hierarchy.
- [ ] Use `agent-browser` for manual browser QA. Existing Playwright specs remain automated regression coverage.
- [ ] Do not modify or revert unrelated uncommitted work in the shared worktree.

## Plane behavior to reproduce

### Shared settings shell

References:

- `references/plane/apps/web/core/components/settings/content-wrapper.tsx`
- `references/plane/apps/web/core/components/settings/control-item.tsx`
- `references/plane/apps/web/core/components/settings/boxed-control-item.tsx`
- `references/plane/apps/web/core/components/settings/sidebar/item.tsx`
- `references/plane/apps/web/core/components/settings/mobile/nav.tsx`

Observed behavior:

- A fixed-width 250px settings sidebar sits beside an independently scrolling content pane.
- Workspace and project settings have a back action, settings title, entity logo/name, and current role in the sidebar header.
- Sidebar links are grouped under labeled categories, divided into compact sections, and filtered by access.
- The active item uses a selected layer while inactive items use a lightweight hover layer.
- Page content is centered at roughly 900px unless a data-heavy page opts into a wider, hugging layout.
- Pages use a compact top header, semantic settings heading, regular control rows, and bordered boxed rows for toggles and destructive actions.
- Workspace and project settings replace the desktop sidebar with a compact mobile menu and active-path label below the breakpoint.

### Bottom profile menu and profile settings

References:

- `references/plane/apps/web/core/components/workspace/sidebar/user-menu-root.tsx`
- `references/plane/apps/web/core/components/settings/profile/sidebar/`
- `references/plane/apps/web/core/components/settings/profile/content/`
- `references/plane/apps/web/app/(all)/settings/profile/`
- `references/plane/packages/constants/src/settings/profile.ts`

Observed behavior:

- The bottom sidebar avatar opens a profile card with cover, avatar, name, and email.
- The menu contains Settings, Preferences, and Sign out. Destructive account actions do not live in this compact menu.
- The full profile page has a 250px sidebar with a user header and grouped items.
- `Your profile` contains Profile, Preferences, Notifications, and Security.
- `Developer` contains API tokens.
- The profile sidebar also lists accessible workspaces, Create workspace, and Workspace invites.
- Plane can render profile settings in a modal, but Jet Black will use URL-driven pages as the canonical surface requested for this project.

### Workspace settings

References:

- `references/plane/packages/constants/src/settings/workspace.ts`
- `references/plane/apps/web/core/components/settings/workspace/sidebar/`
- `references/plane/apps/web/core/components/workspace/settings/workspace-details.tsx`
- `references/plane/apps/web/core/components/workspace/settings/members-list.tsx`
- `references/plane/apps/web/app/(all)/[workspaceSlug]/(settings)/settings/(workspace)/`
- `references/plane/apps/web/core/components/workspace/sidebar/workspace-menu-root.tsx`
- `references/plane/apps/web/core/components/workspace/sidebar/workspace-menu-header.tsx`
- `references/plane/apps/web/core/components/workspace/sidebar/dropdown-item.tsx`

Observed behavior:

- `Administration` contains General, Members, Billing and plans, and Export.
- `Features` is currently empty and is not rendered.
- `Developer` contains Webhooks.
- General settings expose logo, name, organization size, read-only workspace URL, and timezone, followed by deletion controls.
- Members can view the member list; admins can invite, change roles, remove members, and inspect member activity.
- Export is available to workspace admins and members.
- Webhooks are admin-only.
- The active workspace is switchable from the top-left workspace menu, which also exposes Settings, invite shortcuts, Create workspace, Workspace invites, and Sign out according to access.
- The workspace section overflow menu provides Archives and admin-only Settings.

### Project settings

References:

- `references/plane/packages/constants/src/settings/project.ts`
- `references/plane/apps/web/core/components/settings/project/sidebar/`
- `references/plane/apps/web/core/components/project/form.tsx`
- `references/plane/apps/web/core/components/project/settings/`
- `references/plane/apps/web/app/(all)/[workspaceSlug]/(settings)/settings/projects/[projectId]/`
- `references/plane/apps/web/core/components/project-states/`
- `references/plane/apps/web/core/components/labels/project-setting-label-list.tsx`
- `references/plane/apps/web/core/components/workspace/sidebar/projects-list-item.tsx`

Observed behavior:

- `General` contains General and Members.
- `Features` contains separate Cycles, Modules, Views, Pages, and Intake pages.
- `Work structure` contains States, Labels, and Estimates.
- `Execution` contains Automations.
- General settings expose cover, logo, name, description, project identifier, network/visibility, timezone, creation date, archive, and delete.
- Admins manage general settings, feature toggles, estimates, and automations.
- Admins and members can access member, state, and label management where Plane permits it.
- Feature toggles hide or show project navigation without deleting feature data.
- Each project row has an overflow menu with Copy link, Archives, Settings, optional publishing, and Leave project when applicable.

## Intentional Jet Black adaptations

| Plane behavior | Jet Black adaptation |
| --- | --- |
| Routes begin at `/[workspaceSlug]` | Preserve Jet Black's existing `/workspace/[workspaceSlug]` prefix. |
| Profile settings can open as a large modal | Use full URL-driven pages first. A modal is not required for parity in this workstream. |
| Workspace ownership is separate from admin/member/guest membership roles | Add an explicit owner invariant while presenting Plane-aligned `admin`, `member`, and `guest` access levels. |
| Project roles are admin/member/guest | Use exactly `admin`, `member`, and `guest`; do not introduce contributor/commenter aliases. |
| Plane calls the time-box feature Cycles | Map the settings UI to Jet Black's current `sprints` implementation until the broader product terminology is migrated. Keep the mapping explicit in code. |
| Plane exposes billing and commercial surfaces | Omit them until Jet Black has a billing model. Do not render dead sidebar groups or links. |
| Plane exposes API tokens | Omit the Developer profile category until Jet Black has an external API or MCP token contract. |
| Plane has integrations/import surfaces in addition to its current grouped workspace constants | Defer integrations until a connector workflow exists; do not add a nonfunctional settings link. |
| Plane stores first name, last name, and display name separately | Keep Better Auth's `name` authoritative. Match Plane's profile composition without duplicating identity fields in Convex. |
| Plane labels its soft action as account deactivation | Jet Black currently supports hard deletion only. Label it accurately as Delete account unless a real reactivation policy is implemented. |

## Current Jet Black baseline

| Area | Implemented | Missing or partial |
| --- | --- | --- |
| Profile | Better Auth email/password, onboarding name update, sign out, and hard account deletion | No profile menu, profile settings routes, avatar/cover editor, preferences, notifications, password-change UI, or workspace list in profile settings |
| Workspace | Create workspace, one owner/member membership shape, current workspace query, project list | No exact workspace-by-slug selection, switcher, settings routes, update/leave/transfer/delete flow, logo, timezone, organization size, invites, role management, exports, or webhooks |
| Project | Create project, logo/cover storage mutations, states and labels used by issues | No settings routes, update/archive/delete, project membership, role model, feature flags, state settings CRUD, label settings CRUD, estimates configuration, or automations |
| Sidebar | Basic workspace/project navigation and an always-expanded account panel | No Plane-style workspace switcher, overflow menus, bottom avatar popover, profile links, workspace-settings entry, or project-settings entry |
| Authorization | Workspace membership and broad project-through-workspace access helpers | Member enumeration is not membership-gated; there are no role helpers or private-project checks; image mutations allow every workspace member |
| Tests | Auth, navigation, issues, intake, modules, sprints, pages | No settings, permission matrix, invitations, profile preferences, password, archive/delete, export, or webhook coverage |

## Execution order

Complete phases in order. Phase 0 is a release blocker because settings will expose currently unsafe membership and account-deletion paths.

---

## Phase 0: settings-critical safety and route correctness

### 0.1 Authorize workspace member reads

- [ ] In `convex/convex/queries/workspaces.ts`, require membership after authentication and before reading rows in `membersForWorkspace`.
- [ ] Add a focused Convex test proving a member can list their workspace and an authenticated non-member cannot enumerate another workspace's names, emails, or roles.
- [ ] Acceptance: authenticated users cannot query members for a workspace they do not belong to.

### 0.2 Make account deletion safe for shared workspaces

- [ ] Refactor `convex/convex/mutations/accountCleanup.ts` so deleting a non-owner removes only that user's membership and user-owned personal records.
- [ ] Block deletion when the user owns a workspace with other members until ownership is transferred or the workspace is explicitly deleted.
- [ ] Allow complete workspace cleanup only when the deleting user is the sole owner and sole member.
- [ ] Add tests for member deletion, sole-owner deletion, and owner-with-other-members rejection.
- [ ] Acceptance: deleting one account cannot erase another member's workspace, project, work items, comments, or files.

### 0.3 Resolve URL entities exactly

- [ ] Add authorized workspace-by-slug and project-with-access queries.
- [ ] Update SSR loaders to resolve the exact workspace slug and project ID represented by the URL instead of selecting the first membership or project.
- [ ] Reject a project ID that does not belong to the route workspace.
- [ ] Add multi-workspace and cross-workspace project loader tests.
- [ ] Acceptance: settings never preload or flash data from a different workspace or project.

---

## Phase 1: Plane-aligned settings foundation

### 1.1 Add canonical route helpers

- [ ] Extend `apps/web/src/lib/routes.ts` with typed profile, workspace-settings, project-settings, and settings-tab helpers.
- [ ] Use these canonical profile routes:
  - `/settings/profile/general`
  - `/settings/profile/preferences`
  - `/settings/profile/notifications`
  - `/settings/profile/security`
- [ ] Redirect `/settings/profile` to `/settings/profile/general`.
- [ ] Use these canonical workspace routes:
  - `/workspace/[workspaceSlug]/settings`
  - `/workspace/[workspaceSlug]/settings/members`
  - `/workspace/[workspaceSlug]/settings/exports`
  - `/workspace/[workspaceSlug]/settings/webhooks`
  - `/workspace/[workspaceSlug]/settings/webhooks/[webhookId]`
- [ ] Use these canonical project routes, matching Plane's placement under workspace settings:
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/members`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/features/cycles`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/features/modules`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/features/views`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/features/pages`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/features/intake`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/states`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/labels`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/estimates`
  - `/workspace/[workspaceSlug]/settings/projects/[projectId]/automations`
- [ ] Acceptance: components do not hand-build settings URLs.

### 1.2 Centralize settings navigation metadata

- [ ] Add typed profile, workspace, and project settings maps modeled after Plane's `packages/constants/src/settings/*.ts` files.
- [ ] Store key, label, href, icon, category, access requirement, and active-path behavior in those maps.
- [ ] Generate grouped sidebar sections from the maps rather than duplicating links in Svelte components.
- [ ] Filter empty categories instead of rendering empty headings.
- [ ] Acceptance: desktop sidebar, mobile menu, breadcrumbs, and permission checks use one navigation definition per settings domain.

### 1.3 Build shared settings primitives

- [ ] Add focused Svelte primitives under `apps/web/src/lib/components/settings/` for the settings shell, entity header, sidebar, sidebar item, mobile navigation, content wrapper, page heading, control row, boxed control row, and danger zone.
- [ ] Match Plane's 250px sidebar, compact grouped sections, independent content scroll, centered 900px content width, and wider data-page option.
- [ ] Support a back action plus entity logo/avatar, name/email or role, and settings title in the sidebar header.
- [ ] Use semantic headings, labels, descriptions, live error regions, and keyboard-accessible controls.
- [ ] Acceptance: profile, workspace, and project settings share the same primitives without one oversized conditional component.

### 1.4 Add the Plane-aligned permission model

- [ ] Keep workspace ownership as an explicit invariant separate from the membership access level.
- [ ] Expand workspace access levels to `admin`, `member`, and `guest`, migrating the current owner to owner plus admin access and current members to member access.
- [ ] Add `projectMembers` with `admin`, `member`, and `guest` roles.
- [ ] Make workspace owners/admins inherit project-admin access without duplicate project-membership rows.
- [ ] Add focused `requireWorkspaceRole` and `requireProjectRole` helpers.
- [ ] Define each settings route's read and write requirements from the Plane constants before implementing its mutations.
- [ ] Add a permission-matrix test suite for owner, workspace admin, workspace member, workspace guest, project admin, project member, and project guest.
- [ ] Acceptance: role names and page access match Plane; hiding a control is never the only authorization layer.

### 1.5 Add exact settings loaders and types

- [ ] Add layout loaders that authenticate once and resolve the route workspace/project before rendering nested settings pages.
- [ ] Return useful SSR data for the exact settings entity and preserve it during hydration.
- [ ] Derive frontend types from generated Convex and Better Auth contracts in a focused settings type module.
- [ ] Add clear not-found and not-authorized surfaces matching the settings shell.
- [ ] Acceptance: direct settings URLs reload correctly without a loading flash or stale entity data.

---

## Phase 2: Plane-style sidebar entry points

### 2.1 Replace the expanded account panel with a bottom avatar menu

- [ ] Make the bottom sidebar account surface a compact avatar button instead of permanently showing Sign out and Delete account.
- [ ] Open a popover card containing cover treatment, avatar, name, and email.
- [ ] Add Settings, Preferences, and Sign out actions in the same order and hierarchy as Plane.
- [ ] Route Settings to `/settings/profile/general` and Preferences to `/settings/profile/preferences`.
- [ ] Remove Delete account from the compact menu and place it in profile General's danger zone.
- [ ] Preserve loading/error feedback for sign out and close the menu on navigation.
- [ ] Acceptance: profile settings are discoverable from the bottom of the sidebar and destructive actions are not exposed as routine menu items.

### 2.2 Build the top-left workspace switcher

- [ ] Make the current workspace logo/name open a menu listing every accessible workspace.
- [ ] Show each workspace's logo, name, role, member count, active check, and settings shortcut where permitted.
- [ ] Add Create workspace and Workspace invites entries only when those destination flows exist.
- [ ] Switch the URL and all query context when selecting a workspace; never retain the previous workspace's selected project.
- [ ] Add the current workspace's Settings and Invite members quick actions according to access.
- [ ] Acceptance: switching workspaces is URL-driven, realtime-safe, and visually follows Plane's workspace dropdown.

### 2.3 Add workspace section quick actions

- [ ] Add a compact overflow trigger beside the Workspace section heading.
- [ ] Add Archives when the archive surface exists and Settings for workspace admins.
- [ ] Keep the section disclosure and overflow trigger independently keyboard accessible.
- [ ] Acceptance: admins can enter workspace settings from the same sidebar area Plane uses.

### 2.4 Add per-project overflow menus

- [ ] Add a hover/focus-visible overflow trigger to every project row without breaking the row's primary navigation action.
- [ ] Include Copy link, Archives when available, and Settings.
- [ ] Include Leave project for users who may leave and enforce leave invariants in Convex.
- [ ] Route Settings to the project settings root under `/workspace/[workspaceSlug]/settings/projects/[projectId]`.
- [ ] Keep the Settings entry visible to roles that can read at least one project settings page; filter the destination sidebar by access.
- [ ] Acceptance: project settings are discoverable from each project row and menu actions remain usable by keyboard and touch.

### 2.5 Make project module navigation feature-driven

- [ ] Derive project module visibility from project feature flags instead of the static `moduleLinks` list.
- [ ] Preserve work-item access while applying role checks to Cycles/Sprints, Modules, Views, Pages, and Intake.
- [ ] Close mobile/extended navigation after selection and keep active-state behavior URL-driven.
- [ ] Acceptance: toggling a project feature updates every open sidebar in realtime without deleting feature data.

---

## Phase 3: profile settings page

### 3.1 Build the profile settings shell and sidebar

- [ ] Create the authenticated `/settings/profile/[profileTab]` layout with a 250px user sidebar and independently scrolling content pane.
- [ ] Show avatar, name, and email at the top.
- [ ] Add the `Your profile` group with Profile, Preferences, Notifications, and Security in Plane's order.
- [ ] List accessible workspaces beneath settings, followed by Create workspace and Workspace invites only when those flows exist.
- [ ] Omit the empty `Developer` group until API tokens have a real authentication contract.
- [ ] Add a usable mobile settings navigator.
- [ ] Acceptance: all profile tabs are URL-driven, directly reloadable, and preserve Plane's grouped information architecture.

### 3.2 Implement Profile general

- [ ] Match Plane's cover-card composition with cover image, overlapping avatar, identity summary, and form below.
- [ ] Keep Better Auth's `name`, `email`, and `image` authoritative.
- [ ] Allow display-name and avatar updates through Better Auth's update-user API.
- [ ] Display email as read-only until a verified change-email flow is implemented.
- [ ] Store only app-owned profile fields such as cover image or optional job role in a uniquely keyed Convex profile/preferences document.
- [ ] Validate avatar and cover uploads at the boundary for MIME type, size, ownership, and stale-file cleanup.
- [ ] Add save loading, success, field-error, and form-preservation behavior.
- [ ] Add a boxed Delete account danger row using the safe Phase 0 cleanup policy and exact confirmation.
- [ ] Acceptance: profile changes appear in the bottom menu, comments, assignees, and settings header after realtime update or reload.

### 3.3 Implement Preferences

- [ ] Add one `userPreferences` document per Better Auth user.
- [ ] Reproduce Plane's preference rows for theme, timezone, language/locale, and first day of the week.
- [ ] Apply theme before hydration to avoid a color flash.
- [ ] Make calendars and displayed timestamps consume the saved week-start and timezone settings.
- [ ] Save row-level controls immediately with visible success/error feedback.
- [ ] Acceptance: preferences persist across sessions and apply in every workspace.

### 3.4 Implement Notifications

- [ ] Add the email-notification heading and explanatory copy.
- [ ] Add Plane-aligned toggles for property changes, state changes, completed work items, comments, and mentions.
- [ ] Render completed-work-item notifications as a child of state-change notifications and disable them when the parent is disabled.
- [ ] Persist preferences even before outbound email delivery exists, while clearly labeling them as email preferences.
- [ ] Acceptance: each toggle saves immediately and reloads accurately.

### 3.5 Implement Security

- [ ] Add current password when required, new password, confirm password, show/hide controls, and password-strength guidance.
- [ ] Use Better Auth's change-password API; do not expose credential or account records through Convex queries.
- [ ] Reject a reused current password, mismatched confirmation, and Better Auth policy failures without clearing the form prematurely.
- [ ] Define and test the selected session-revocation behavior.
- [ ] Acceptance: the old password fails and the new password succeeds after a completed change.

### 3.6 Defer unsupported profile surfaces explicitly

- [ ] API tokens: defer until Jet Black has an external API or MCP authentication contract.
- [ ] Social connections: defer until OAuth providers are configured in Better Auth.
- [ ] Account deactivation: defer until there is a documented reactivation policy; do not mislabel hard deletion as deactivation.
- [ ] Profile settings modal: defer until the full URL-driven page is complete and stable.

---

## Phase 4: workspace settings

### 4.1 Build the workspace settings shell and navigation

- [ ] Add the Plane-style back action, `Workspace settings` title, workspace logo/name, and current role to the sidebar header.
- [ ] Add `Administration` with General, Members, and Export.
- [ ] Add `Developer` with Webhooks for admins.
- [ ] Omit Billing and plans, empty Features, and Integrations until those products exist.
- [ ] Apply Plane-aligned access filtering to both navigation and route loaders.
- [ ] Acceptance: sidebar categories, ordering, active states, mobile behavior, and access match the local Plane constants after documented omissions.

### 4.2 Implement Workspace general

- [ ] Extend workspace data with optional logo, organization size, timezone, and explicit owner fields.
- [ ] Keep the workspace slug/URL read-only to match Plane's current general settings form.
- [ ] Add authorized updates for logo, name, organization size, and timezone.
- [ ] Match Plane's logo/name/URL summary followed by compact responsive form fields and Update workspace action.
- [ ] Add an admin-only boxed Delete workspace row with exact-name confirmation.
- [ ] Acceptance: workspace details update in realtime throughout the switcher, sidebar, and settings header.

### 4.3 Implement Members and invitations

- [ ] Add workspace invitations with email, role, inviter, token hash, expiry, status, and timestamps.
- [ ] Add authorized list, invite, resend, revoke, and accept functions.
- [ ] Match Plane's members page with count, search, role filter, activity action, invite action, pending invitations, and member rows.
- [ ] Allow admins to change roles and remove members while members can view the page.
- [ ] Prevent removing or demoting the final owner and prevent actors from managing a role above their authority.
- [ ] Add a development copy-link fallback when email delivery is not configured.
- [ ] Acceptance: invitations cannot be accepted twice, after expiry, by the wrong email, or into the wrong workspace.

### 4.4 Implement ownership, leave, and deletion invariants

- [ ] Add explicit ownership transfer.
- [ ] Allow non-owners to leave without deleting shared data.
- [ ] Block the owner from leaving until ownership is transferred.
- [ ] Allow only the owner to delete the workspace.
- [ ] Use bounded or scheduled cleanup if workspace deletion exceeds one Convex transaction.
- [ ] Redirect affected clients after leave or deletion.
- [ ] Acceptance: ownership and deletion invariants are enforced entirely by Convex.

### 4.5 Implement Export

- [ ] Match Plane's export heading and guide flow.
- [ ] Support JSON and CSV first with all-project or single-project scope.
- [ ] Authorize every exported project and exclude private projects the requester cannot access.
- [ ] Track export status/history and store downloadable files with expiry and cleanup.
- [ ] Allow workspace admins and members, matching Plane's current access constants.
- [ ] Acceptance: generated exports are bounded, downloadable, and contain no inaccessible data.

### 4.6 Implement Webhooks

- [ ] Add admin-only webhook list, compact empty state, Create webhook action, and webhook detail route.
- [ ] Store URL, secret strategy, enabled events, active state, creator, and timestamps.
- [ ] Reveal a newly generated secret only at creation/rotation time.
- [ ] Validate HTTPS destinations and prevent SSRF to private or local network targets.
- [ ] Deliver through Convex actions with bounded retries and delivery logs.
- [ ] Acceptance: only admins can create, rotate, disable, or delete webhooks, and failed deliveries are inspectable without exposing secrets.

### 4.7 Keep unsupported workspace surfaces out of navigation

- [ ] Billing, plans, seats, and licenses: defer until Jet Black has a billing model.
- [ ] Integrations/importers: defer until at least one connector workflow is defined end to end.
- [ ] SSO, SAML, and IdP sync: defer until enterprise authentication is requested.
- [ ] Empty feature categories: do not render them merely to resemble Plane's constants.

---

## Phase 5: project settings

### 5.1 Build the project settings shell and navigation

- [ ] Add the Plane-style back action to project work items, `Project settings` title, project logo/name, and current project role.
- [ ] Add `General`: General and Members.
- [ ] Add `Features`: Cycles, Modules, Views, Pages, and Intake as separate pages.
- [ ] Add `Work structure`: States, Labels, and Estimates.
- [ ] Add `Execution`: Automations.
- [ ] Filter each item using the Plane-aligned project role matrix.
- [ ] Acceptance: categories, order, active matching, mobile behavior, and access match `references/plane/packages/constants/src/settings/project.ts`.

### 5.2 Implement Project general

- [ ] Extend project data with network/visibility, timezone, archived timestamp, creation timestamp, and explicit feature flags.
- [ ] Add authorized project update, archive, restore, and delete functions.
- [ ] Reuse existing logo/cover storage paths but add MIME, size, ownership, replacement, and stale-file validation.
- [ ] Match Plane's cover header with logo picker, project name, identifier, visibility, and Change cover action.
- [ ] Match Plane's fields: name, description, project identifier, network/visibility, and timezone.
- [ ] Show creation date beside Update project.
- [ ] Keep issue identifiers historically stable if the project identifier changes unless a separate migration is approved.
- [ ] Add boxed Archive and Delete rows; deletion requires exact project-name confirmation.
- [ ] Acceptance: project cards, sidebar, headers, and settings update in realtime; archived projects leave active navigation but remain restorable.

### 5.3 Implement Project members

- [ ] Add project membership list, add/remove, and role changes for `admin`, `member`, and `guest`.
- [ ] Let workspace owners/admins retain project-admin access without duplicate rows.
- [ ] Add Plane-aligned default member settings that Jet Black can support, such as default assignee and default state.
- [ ] Public projects may be discoverable to workspace members; private projects require explicit membership or inherited workspace-admin access.
- [ ] Enforce final-project-admin and guest access invariants.
- [ ] Omit Plane teamspaces until Jet Black implements that separate product concept.
- [ ] Acceptance: private project data and settings are inaccessible without project access.

### 5.4 Implement separate feature settings pages

- [ ] Add one boxed control page each for Cycles/Sprints, Modules, Views, Pages, and Intake.
- [ ] Use the same shared feature-control component with page-specific title, description, icon, property, and access.
- [ ] Keep the Cycles-to-Sprints mapping explicit until terminology is unified.
- [ ] Disabling a feature hides its navigation and skips its queries without deleting existing data.
- [ ] Show an admin-aware disabled state for deep links instead of a blank page.
- [ ] Acceptance: each feature change propagates to open clients in realtime and survives reload.

### 5.5 Implement States

- [ ] Add state create, rename, recolor, change-group, reorder, and mark-default functions.
- [ ] Group the UI by backlog, unstarted, started, completed, and cancelled using Plane's project-state components as the interaction reference.
- [ ] Require exactly one default state per project.
- [ ] When deleting an in-use state, require a replacement and migrate affected work items safely.
- [ ] Prevent deleting the final valid state.
- [ ] Acceptance: board/list order and new-work-item defaults update immediately.

### 5.6 Implement Labels

- [ ] Add centralized label create, rename, recolor, reorder, and delete behavior modeled on Plane's settings list.
- [ ] Prevent duplicate normalized names inside one project.
- [ ] Remove assignments safely before deleting a label.
- [ ] Keep nested label groups out of the first parity slice unless the current Plane settings interaction requires them.
- [ ] Acceptance: label edits and deletion update detail, board, and filter UI in realtime.

### 5.7 Implement Estimates

- [ ] Add one active estimate system per project.
- [ ] Support points first while keeping the schema able to represent category or time systems later.
- [ ] Add enable/disable and allowed-value configuration through the project settings page.
- [ ] Do not replace existing free-form estimates until a data migration is defined.
- [ ] Acceptance: the work-item estimate picker is driven by project configuration rather than hardcoded values.

### 5.8 Implement Automations

- [ ] Add Plane-aligned auto-close and auto-archive controls with inactivity windows and target states.
- [ ] Use Convex scheduled/internal functions rather than workers or external queues.
- [ ] Store last run, next run, status, and idempotency information.
- [ ] Keep custom automations and due-date reminders deferred until their dependent notification/rule systems exist.
- [ ] Acceptance: automation runs are bounded, retry-safe, authorized, and covered by deterministic tests.

---

## Phase 6: verification and release gates

### 6.1 Static and automated checks

- [ ] `bun run --cwd convex typecheck`
- [ ] `bun run --cwd convex test`
- [ ] `bun run --cwd apps/web typecheck`
- [ ] `bun run --cwd apps/web build`
- [ ] `bun x ultracite check`
- [ ] Audit handwritten source for explicit `any`, type suppressions, `console.log`, `debugger`, and duplicated Convex domain shapes.

### 6.2 Authorization coverage

- [ ] Cross-workspace member enumeration is denied.
- [ ] Workspace update/delete/member actions are denied to insufficient roles.
- [ ] Project settings and private project reads are denied without project access.
- [ ] Final owner and project-admin invariants are enforced.
- [ ] Invite expiry, email binding, replay prevention, and revocation are enforced.
- [ ] Account deletion cannot remove another user's shared data.
- [ ] Webhook and export reads cannot cross workspace/project boundaries.

### 6.3 Browser scenarios

- [ ] Bottom avatar menu opens, shows identity, and routes to Profile and Preferences.
- [ ] Profile name/avatar/cover/preferences persist and appear throughout the UI.
- [ ] Password change succeeds; old password fails and new password succeeds.
- [ ] Workspace switching changes URL and context without flashing prior workspace data.
- [ ] Workspace member invite, role change, removal, leave, and ownership transfer work.
- [ ] Workspace general settings update the switcher and settings header.
- [ ] Project general settings update cards, sidebar, and headers.
- [ ] Project feature toggles hide/show navigation without deleting data.
- [ ] State and label management update work-item behavior.
- [ ] Archive/restore and every destructive confirmation flow work.
- [ ] Direct profile, workspace-settings, and project-settings URLs survive reload.

### 6.4 Visual and accessibility QA

- [ ] Capture desktop and mobile screenshots for profile, workspace, and project settings.
- [ ] Compare sidebar width, category rhythm, entity headers, content width, control rows, boxed rows, and danger zones against the local Plane reference.
- [ ] Verify settings navigation collapses into a usable mobile control.
- [ ] Verify long names, emails, timezones, and localized strings do not clip.
- [ ] Verify menus, dialogs, uploads, toggles, and destructive flows are keyboard accessible and visibly focused.
- [ ] Verify loading, empty, error, unauthorized, and disabled-feature states.

### 6.5 Documentation and cleanup

- [ ] Update the settings sections in `TODO.md` after each completed phase so the two files do not report conflicting status.
- [ ] Document role semantics, inherited project access, ownership transfer, and destructive-operation policies in a short architecture note.
- [ ] Remove temporary accounts, uploads, exports, webhook deliveries, and screenshots.
- [ ] Run `review-work` and `visual-qa`; resolve every blocking finding before completion.

## Definition of done

The settings workstream is complete only when:

1. The bottom profile menu, workspace switcher, workspace quick actions, and project overflow menus expose the same settings entry hierarchy as Plane after documented omissions.
2. Profile, workspace, and project settings use Plane's grouped navigation, content hierarchy, responsive behavior, and permission visibility.
3. All settings are URL-driven and resolve the exact workspace/project represented by the URL.
4. Convex enforces every protected read, write, membership action, export, webhook action, and destructive operation.
5. Better Auth remains authoritative for identity, credentials, sessions, and hard account deletion.
6. Frontend types derive from generated contracts without duplicated backend shapes or type suppressions.
7. Shared-workspace account deletion cannot erase another user's data.
8. Typecheck, build, lint, backend tests, browser QA, `review-work`, and `visual-qa` pass.
