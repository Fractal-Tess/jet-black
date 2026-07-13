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
