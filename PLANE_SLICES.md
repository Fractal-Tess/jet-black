# Plane Slice Briefs

This file captures the Plane UI reference pass before each Jet Black slice.
Each slice should include a compact wireframe, user actions, visible reactions,
and implementation notes for Jet Black.

## Intake

Plane reference files:

- `references/plane/apps/web/app/(all)/[workspaceSlug]/(projects)/projects/(detail)/[projectId]/intake/page.tsx`
- `references/plane/apps/web/app/(all)/[workspaceSlug]/(projects)/projects/(detail)/[projectId]/intake/layout.tsx`
- `references/plane/apps/web/ce/components/projects/settings/intake/header.tsx`
- `references/plane/packages/types/src/inbox.ts`
- `references/plane/packages/constants/src/intake.ts`

Wireframe:

```text
┌─ App shell ────────────────────────────────────────────────────────────────┐
│ sidebar project nav                         top project/module breadcrumb │
├────────────────────────────────────────────────────────────────────────────┤
│ Intake header                                                               │
│ ┌─ New intake item ───────────────────────────────────────────────────────┐ │
│ │ title input                                                             │ │
│ │ source input                                                            │ │
│ │ description textarea                                                    │ │
│ │ [Add intake item]                                                       │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│ ┌─ Inbox / triage list ───────────────────────────────────────────────────┐ │
│ │ [pending badge] [source] title                         [Decline][Accept]│ │
│ │ description                                                             │ │
│ │ [accepted badge] [source] title                         Accepted as issue│ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────┘
```

Actions and reactions:

- Create intake item:
  - validates non-empty title.
  - inserts a pending item.
  - immediately appears in the realtime inbox list.
  - clears the form.
- Accept:
  - creates a normal issue using the project sequence/key.
  - marks intake item as accepted and stores the created issue id.
  - navigates to the accepted issue in the normal Issues module.
- Decline:
  - marks intake item declined.
  - keeps it visible with declined status for auditability.
- Empty state:
  - show a quiet card when no intake items exist.

Style notes to copy:

- Keep intake inside the same project shell and module navigation.
- Use dense cards, small uppercase section labels, compact status pills, and
  right-aligned row actions.
- Avoid a separate backend queue; Convex mutations perform create/accept/decline
  directly and subscriptions update the UI.

## Issues and work items

Plane reference files:

- `references/plane/apps/web/core/components/issues/issue-layouts/list/block.tsx`
- `references/plane/apps/web/core/components/issues/issue-layouts/kanban/block.tsx`
- `references/plane/apps/web/core/components/issues/filters.tsx`
- `references/plane/apps/web/core/components/issues/issue-layouts/properties/all-properties.tsx`
- `references/plane/apps/web/core/components/issues/peek-overview/root.tsx`
- `references/plane/apps/web/core/components/issues/issue-detail/root.tsx`
- `references/plane/apps/web/core/components/issues/archive-issue-modal.tsx`
- `references/plane/apps/web/core/components/issues/delete-issue-modal.tsx`
- `references/plane/apps/web/core/components/issues/issue-detail/reactions/issue.tsx`

Wireframe:

```text
Issues header
[List|Board|Calendar|Sheet|Gantt] [Filters] [Display] [Analytics] [Add work item]

Board
┌ Todo 12 + ──────┐ ┌ In progress 4 + ─┐ ┌ Done 8 + ─────┐
│ KEY-123      …  │ │ KEY-130       …  │ │ KEY-111    …  │
│ Work item title │ │ Work item title  │ │ Work item     │
│ [State][P][👤]  │ │ [Due][Label]     │ │ [P][Cycle]    │
│ + New work item │ │ + New work item  │ │ + New work    │
└─────────────────┘ └──────────────────┘ └───────────────┘

Detail/peek
┌ close full-mode saved link more ┐
│ editable title                   │
│ rich description                 │
│ reactions                        │
│ widgets: sub-items/relations     │
│ comments/activity                │
├ properties sidebar ──────────────┤
│ state priority assignees labels  │
│ cycle module dates parent        │
└──────────────────────────────────┘
```

Actions and reactions:

- Row/card click opens the work item; nested dropdowns stop propagation.
- Hover reveals checkbox, quick properties, and overflow actions.
- Right-click opens the same actions as overflow.
- Quick create is contextual to group/column.
- Archive/delete use confirmation modals and close/redirect from detail.
- Reactions render as emoji pills under description; clicking toggles the
  current user reaction.
- Filters and display settings are separate controls.
- Display settings control visible properties, grouping, ordering, empty groups,
  and sub-issue visibility.

Style notes to copy:

- Whole row/card should feel clickable.
- Work item properties should be compact chips, not full form controls.
- Board cards are dense: key, title, wrapped property chips, subtle elevation.
- List rows are low-height, border-separated, and hover-reveal actions.
- Detail should eventually support side peek, modal, and full-screen modes.
- Reactions belong close to description, not buried in comments.

## Cycles

Plane reference files:

- `references/plane/apps/web/core/components/cycles/cycles-view.tsx`
- `references/plane/apps/web/core/components/cycles/list/cycles-list-item.tsx`
- `references/plane/apps/web/core/components/cycles/cycle-peek-overview.tsx`
- `references/plane/apps/web/core/components/cycles/analytics-sidebar/root.tsx`
- `references/plane/apps/web/core/components/cycles/form.tsx`
- `references/plane/apps/web/core/components/cycles/quick-actions.tsx`

Wireframe:

```text
Cycles list
┌ Header: Project / Cycles                 [Filters] [Add cycle] ┐
│ Active cycle block                                             │
│ Upcoming ▾ count                                               │
│  ◌ progress  title  date range  issue count  favorite  …       │
│ Completed ▾ count                                              │
│  ✓ progress  title  date range  issue count  favorite  …       │
└────────────────────────────────────────────────────────────────┘

Cycle detail
┌ Header: cycle switcher count [layout] [filters] [display] [+] ┐
├ work-item board/list/calendar/sheet/gantt ┬ analytics sidebar ┤
│ issues assigned to cycle                  │ status/date        │
│                                           │ progress/stats     │
└───────────────────────────────────────────┴────────────────────┘
```

Actions and reactions:

- Add/edit cycle opens a modal with title, description, and date range.
- List item click opens detail; info/eye opens a peek analytics sidebar.
- Groups expand/collapse with sticky headers.
- Favorite toggles with toast.
- Archive is disabled unless cycle is completed.
- Completed cycles can transfer unfinished issues.
- Detail sidebar collapse persists locally.
- Analytics stat clicks can update work-item filters.

Style notes to copy:

- Use circular progress and status-colored soft badges.
- Cycle rows should be dense: progress ring, title, issue count, dates, actions.
- Keep analytics sidebar narrow and scannable.

## Modules

Plane reference files:

- `references/plane/apps/web/core/components/modules/modules-list-view.tsx`
- `references/plane/apps/web/core/components/modules/module-list-item.tsx`
- `references/plane/apps/web/core/components/modules/module-card-item.tsx`
- `references/plane/apps/web/core/components/modules/module-peek-overview.tsx`
- `references/plane/apps/web/core/components/modules/analytics-sidebar/root.tsx`
- `references/plane/apps/web/core/components/modules/links/list.tsx`
- `references/plane/apps/web/core/components/modules/form.tsx`

Wireframe:

```text
Modules list
┌ Header: Project / Modules       [view] [filters] [order] [Add] ┐
│ list: progress title date status lead favorite …               │
│ board card: title status info                                  │
│             issue count lead                                   │
│             segmented progress bar                             │
│             date range favorite …                              │
└────────────────────────────────────────────────────────────────┘

Module detail
┌ Header: module switcher count [layout] [filters] [display] [+] ┐
├ module issues layout                       ┬ module sidebar     │
│                                            │ status/title/desc   │
│                                            │ lead/members/dates  │
│                                            │ progress/links      │
└────────────────────────────────────────────┴────────────────────┘
```

Actions and reactions:

- Add/edit module modal: title, description, status, date range, lead, members.
- Layout switch supports list/board/gantt on module list.
- Card/list click opens detail; info opens peek sidebar.
- Inline status/date/lead/member controls save with toasts.
- Archive is disabled unless module is completed or cancelled.
- Links support create/edit/open/copy/delete.
- Detail add work item creates an issue bound to the module.

Style notes to copy:

- Modules are more portfolio/card-like than cycles.
- Use segmented progress bars on module cards.
- Sidebar should be wider than cycle sidebar because modules carry links and
  editable metadata.

## Views

Plane reference files:

- `references/plane/apps/web/app/(all)/[workspaceSlug]/(projects)/projects/(detail)/[projectId]/views/(list)/page.tsx`
- `references/plane/apps/web/app/(all)/[workspaceSlug]/(projects)/projects/(detail)/[projectId]/views/(list)/header.tsx`
- `references/plane/packages/constants/src/views.ts`

Wireframe:

```text
Views list
┌ Project / Views                                      [Add view] ┐
│ Search views                                                    │
│ Default views                                                   │
│ Custom views                                                    │
└─────────────────────────────────────────────────────────────────┘

View detail
┌ Views / Current view [layout] [filters] [display] [Add view] ┐
│ applied filters, only when active                             │
│ work-item layout                                              │
└───────────────────────────────────────────────────────────────┘
```

Actions and reactions:

- Search filters the view list inline.
- Add view opens a modal.
- Breadcrumb dropdown switches views.
- Filter/display/layout changes update the saved view state.
- Disabled feature state points admins to project feature settings.

Style notes to copy:

- Keep list route and detail route separate.
- Use compact search strip and far-right primary CTA.

## Pages

Plane reference files:

- `references/plane/apps/web/core/components/pages/list/root.tsx`
- `references/plane/apps/web/core/components/pages/pages-list-main-content.tsx`
- `references/plane/apps/web/core/components/pages/navigation-pane/root.tsx`
- `references/plane/apps/web/core/components/pages/navigation-pane/tabs-list.tsx`

Wireframe:

```text
Pages list
┌ Project / Pages                         [Add page] ┐
│ [Public | Private | Archived]                       │
│ page rows                                            │
└──────────────────────────────────────────────────────┘

Page editor
┌ document editor ───────────────────┐┌ right inspector ┐
│ title/content                      ││ tabs            │
│                                    ││ outline/info    │
│                                    ││ assets          │
└────────────────────────────────────┘└─────────────────┘
```

Actions and reactions:

- Add page creates public/private based on current tab and navigates to detail.
- Right inspector opens/closes with width animation.
- Inspector tab state lives in query params.
- Empty states distinguish no pages, archived pages, and no search matches.

Style notes to copy:

- Copy the document + side inspector pattern.
- Use query params for pane/tab state.

## Project settings

Plane reference files:

- `references/plane/apps/web/core/components/project-states/root.tsx`
- `references/plane/apps/web/core/components/project-states/group-item.tsx`
- `references/plane/apps/web/core/components/project-states/state-item.tsx`
- `references/plane/apps/web/core/components/labels/project-setting-label-list.tsx`
- `references/plane/apps/web/core/components/settings/project/content/feature-control-item.tsx`
- `references/plane/apps/web/core/components/settings/content-wrapper.tsx`
- `references/plane/apps/web/core/components/settings/boxed-control-item.tsx`

Wireframe:

```text
Settings page
┌ centered settings content ────────────┐
│ title + description                   │
│ boxed control rows                    │
└───────────────────────────────────────┘

States
[Backlog +]   state rows hover: default/edit/delete
[Unstarted +]
[Started +]
[Completed +]
[Cancelled +]

Labels
[Add label]
label group
  child label rows
single label rows
```

Actions and reactions:

- Settings pages permission-gate before rendering.
- State groups expand/collapse and support inline create.
- State rows support reorder, edit, delete, and default state controls.
- Labels support nested groups, inline create/edit, drag grouping, delete modal.
- Feature toggles optimistically update and toast.

Style notes to copy:

- Use centered settings wrappers for focused settings.
- Use boxed rows with title/copy left and control right.
- Keep destructive controls hover-only.

## Workspace home and stickies

Plane reference files:

- `references/plane/apps/web/core/components/home/root.tsx`
- `references/plane/apps/web/core/components/home/home-dashboard-widgets.tsx`
- `references/plane/apps/web/core/components/home/widgets/manage/widget-list.tsx`
- `references/plane/apps/web/core/components/stickies/layout/stickies-list.tsx`
- `references/plane/apps/web/core/components/stickies/sticky/root.tsx`
- `references/plane/apps/web/core/components/stickies/action-bar.tsx`

Wireframe:

```text
Workspace home
┌ greeting / manage widgets ┐
│ quick links               │
│ recents                   │
│ my stickies               │
└───────────────────────────┘

Stickies
┌ Stickies                  [Search] [Add sticky] ┐
│ masonry note grid                               │
│ infinite-scroll sentinel                         │
└ floating action bar: all / recent / + / sticky ─┘
```

Actions and reactions:

- Home widgets can be shown/hidden/reordered.
- Stickies search filters notes.
- Add sticky creates and opens a note.
- Masonry reflows on content change.
- Drag/drop changes sticky position.
- Floating action bar expands with animation.

Style notes to copy:

- Home should feel like a daily cockpit, not a generic dashboard.
- Stickies should use masonry density and colored cards against the dark canvas.
