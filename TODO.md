# Jet Black Plane-style Product TODO

Goal: build a lean Plane-style project tracker with realtime Convex data, keeping the product surface familiar while avoiding Plane's heavier backend, workers, API service, caches, and sync layers.

Stack note: the app is SvelteKit + Convex. The earlier Next.js mention was a misstatement; continue implementing in SvelteKit.

## Non-negotiable product constraints

- [ ] Use Convex as the only backend/data/function layer.
- [ ] Do not add workers, queues, REST API services, Django, Redis, Celery, or Plane-style sync services.
- [ ] Keep realtime behavior native to Convex subscriptions.
- [ ] Keep files small and module-scoped.
- [ ] Use Playwright for e2e tests by controlling the browser UI only.
- [ ] Keep the UI visually close to Plane: dark shell, dense sidebars, command/search bar, project module navigation, compact issue cards, kanban columns, detail drawers.

## Plane reference surfaces to mimic

Observed in `references/plane`:

- Workspace home
- Projects list and project detail
- Work items / issues
- Kanban, list, and richer issue layouts
- Issue detail page/drawer
- Comments, activity, properties, states, labels, assignees
- Intake / draft issues
- Cycles
- Modules
- Views
- Pages
- Archives
- Stickies
- Notifications
- Members and permissions
- Project settings: states, labels, members, feature toggles
- Workspace settings: members, integrations, exports, webhooks
- Search / command palette
- Profile pages and assigned/user issue views
- Analytics

## Current Jet Black baseline

- [x] Email/password auth.
- [x] Session-gated dashboard.
- [x] Account deletion through UI.
- [x] Workspace creation for new users.
- [x] Project creation.
- [x] Issue creation.
- [x] Issue update.
- [x] Issue comments.
- [x] Issue states.
- [x] Issue labels.
- [x] Basic project sidebar.
- [x] Convex schema split into focused files.
- [x] Convex queries/mutations folders.
- [x] Dev seed/init.
- [x] Playwright e2e tests for auth/session/issues.

## Phase 0: product and stack cleanup

- [x] Decide SvelteKit vs Next.js before adding routes.
- [x] Remove leftover worker package if still present and unused.
- [ ] Make the shell route structure match the product model:
  - [ ] `/` redirects based on auth.
  - [ ] `/login`
  - [ ] `/workspace/:workspaceSlug`
  - [ ] `/workspace/:workspaceSlug/projects`
  - [ ] `/workspace/:workspaceSlug/projects/:projectId/issues`
  - [ ] `/workspace/:workspaceSlug/projects/:projectId/issues/:issueId`
  - [ ] `/workspace/:workspaceSlug/projects/:projectId/intake`
  - [ ] `/workspace/:workspaceSlug/projects/:projectId/cycles`
  - [ ] `/workspace/:workspaceSlug/projects/:projectId/modules`
  - [ ] `/workspace/:workspaceSlug/projects/:projectId/views`
  - [ ] `/workspace/:workspaceSlug/projects/:projectId/pages`
- [ ] Replace dashboard-only local selected-state with URL-driven selection.
- [ ] Add typed route helpers for workspace/project URLs.
- [ ] Add empty-state pages for every planned project module.

## Phase 1: Plane-like app shell

- [ ] Rework sidebar to match Plane layout:
  - [ ] Workspace switcher top left.
  - [ ] New work item button.
  - [ ] Home.
  - [ ] Stickies placeholder.
  - [ ] Workspace section.
  - [ ] Projects section with project list.
  - [ ] Project module nav when inside a project.
- [ ] Add top command/search bar.
- [ ] Add user/account menu.
- [ ] Add realtime connection indicator.
- [ ] Add responsive mobile sidebar.
- [ ] Add keyboard shortcuts scaffold:
  - [ ] Open command palette.
  - [ ] New issue.
  - [ ] Search.
- [ ] Add e2e coverage for shell navigation and auth-gated routes.

## Phase 2: kanban MVP

- [x] Build Convex mutation path to move an issue between states.
- [x] Add `position` or `sortOrder` to issues for ordering inside a state.
- [x] Add mutation to reorder within a state.
- [x] Add mutation to reorder across states.
- [x] Build kanban board grouped by issue state.
- [x] Build Plane-style kanban card:
  - [x] Identifier.
  - [x] Title.
  - [x] Priority.
  - [x] State.
  - [x] Assignee avatar placeholder.
  - [x] Comment count.
  - [ ] Module/cycle badges once those exist.
- [x] Add quick issue creation inside each state column.
- [x] Add drag-and-drop issue movement.
- [ ] Add optimistic UI behavior where safe.
- [x] Ensure updates appear in another browser context without refresh.
- [x] Add Playwright realtime test with two browser contexts.
- [x] Add Playwright kanban drag/drop tests.

## Phase 3: issue model parity

- [ ] Extend issue schema:
  - [ ] `labelIds`.
  - [ ] `moduleIds`.
  - [ ] `cycleId`.
  - [x] `parentIssueId`.
  - [x] `estimate`.
  - [x] `startDate`.
  - [x] `targetDate`.
  - [x] `archivedAt`.
  - [x] `completedAt`.
  - [x] `createdAt`.
- [x] Add issue labels table and mutations.
- [x] Add issue attachments placeholder model.
- [ ] Add issue links model.
- [ ] Add issue relations model:
  - [ ] blocks.
  - [ ] blocked by.
  - [ ] relates to.
  - [ ] duplicates.
- [ ] Add sub-issues.
- [ ] Add issue activity feed with structured event types.
- [ ] Add rich issue detail drawer/page:
  - [ ] Title.
  - [ ] Description.
  - [ ] State.
  - [ ] Priority.
  - [ ] Assignee.
  - [x] Labels.
  - [ ] Cycle.
  - [ ] Modules.
  - [x] Dates.
  - [ ] Comments.
  - [ ] Activity.
- [x] Add issue delete/archive.
- [x] Add issue search by title/identifier.
- [ ] Add e2e coverage for all issue properties.

## Phase 4: list view

- [ ] Add project issues list layout.
- [ ] Add sortable columns:
  - [ ] Identifier.
  - [ ] Title.
  - [ ] State.
  - [ ] Priority.
  - [ ] Assignee.
  - [ ] Updated.
- [ ] Add filters:
  - [ ] State.
  - [ ] Priority.
  - [ ] Assignee.
  - [ ] Label.
  - [ ] Module.
  - [ ] Cycle.
- [ ] Add group-by controls:
  - [ ] State.
  - [ ] Priority.
  - [ ] Assignee.
  - [ ] Module.
  - [ ] Cycle.
- [ ] Persist view preferences per user/project.
- [ ] Add e2e tests for filtering, sorting, and grouping.

## Phase 5: project settings

- [ ] Project overview settings.
- [ ] Project members list.
- [ ] Invite/add member flow.
- [ ] Project member roles.
- [ ] State management:
  - [ ] Create state.
  - [ ] Rename state.
  - [ ] Reorder states.
  - [ ] Change state color/type.
  - [ ] Delete/merge state.
- [ ] Label management:
  - [ ] Create label.
  - [ ] Rename label.
  - [ ] Change color.
  - [ ] Delete label.
- [ ] Feature toggles:
  - [ ] Cycles enabled.
  - [ ] Modules enabled.
  - [ ] Views enabled.
  - [ ] Pages enabled.
  - [ ] Intake enabled.
- [ ] E2E coverage for settings changes affecting the UI.

## Phase 6: intake

- [ ] Add intake issue schema:
  - [ ] title.
  - [ ] description.
  - [ ] source.
  - [ ] status: pending, accepted, declined, snoozed.
  - [ ] projectId.
  - [ ] createdByUserId.
- [ ] Add project intake page.
- [ ] Add create intake item.
- [ ] Add accept intake item into real issue.
- [ ] Add decline intake item.
- [ ] Add intake count badge in project nav.
- [ ] Add e2e test for intake -> accepted issue.

## Phase 7: cycles

- [ ] Add cycles schema:
  - [ ] name.
  - [ ] description.
  - [ ] startDate.
  - [ ] endDate.
  - [ ] status derived from dates.
  - [ ] projectId.
- [ ] Add issue `cycleId`.
- [ ] Add cycles list page.
- [ ] Add cycle detail issue board/list.
- [ ] Add add/remove issue from cycle.
- [ ] Add active cycle shortcut.
- [ ] Add cycle progress stats.
- [ ] Add e2e coverage for creating a cycle and moving issues into it.

## Phase 8: modules

- [ ] Add modules schema:
  - [ ] name.
  - [ ] description.
  - [ ] status.
  - [ ] leadUserId.
  - [ ] startDate.
  - [ ] targetDate.
  - [ ] projectId.
- [ ] Add module issue join table or `moduleIds` strategy.
- [ ] Add modules list page.
- [ ] Add module detail issue board/list.
- [ ] Add add/remove issue from module.
- [ ] Add module progress stats.
- [ ] Add e2e coverage for creating a module and assigning issues to it.

## Phase 9: views

- [ ] Add saved views schema:
  - [ ] name.
  - [ ] projectId or workspaceId.
  - [ ] filters.
  - [ ] groupBy.
  - [ ] orderBy.
  - [ ] layout.
- [ ] Add views list page.
- [ ] Add create view from current filters.
- [ ] Add view detail page that reuses issue list/kanban components.
- [ ] Add edit/delete saved view.
- [ ] Add e2e coverage for saved filters.

## Phase 10: pages

- [ ] Add pages schema:
  - [ ] title.
  - [ ] content.
  - [ ] projectId.
  - [ ] createdByUserId.
  - [ ] updatedAt.
- [ ] Add pages list page.
- [ ] Add page detail editor.
- [ ] Use simple markdown/plain text first.
- [ ] Add issue embeds/links later.
- [ ] Add e2e coverage for create/edit/delete page.

## Phase 11: workspace home and global views

- [ ] Add home dashboard:
  - [ ] greeting.
  - [ ] quick links.
  - [ ] recent issues.
  - [ ] assigned to me.
  - [ ] recently updated projects.
- [ ] Add workspace issue browse page.
- [ ] Add assigned-to-me view.
- [ ] Add created-by-me view.
- [ ] Add archived issues page.
- [ ] Add stickies placeholder or simple sticky notes.
- [ ] Add notifications placeholder.
- [ ] Add e2e coverage for home and global issue views.

## Phase 12: command palette and search

- [ ] Add command palette component.
- [ ] Search projects.
- [ ] Search issues by title/identifier.
- [ ] Jump to project module pages.
- [ ] Create issue from palette.
- [ ] Add keyboard shortcut tests where practical.

## Phase 13: realtime hardening

- [ ] Test multi-tab issue creation appears live.
- [ ] Test multi-tab kanban movement appears live.
- [ ] Test comment creation appears live.
- [ ] Handle deleted selected issue gracefully.
- [ ] Handle lost/recovered Convex websocket state.
- [ ] Add visible realtime stale/error states.

## Phase 14: PR/code-review future extension

- [ ] Add repositories table.
- [ ] Add pull requests table.
- [ ] Add issue to pull request relation.
- [ ] Add PR status fields:
  - [ ] branch.
  - [ ] author.
  - [ ] reviewers.
  - [ ] checks.
  - [ ] merge state.
- [ ] Add PR tab in issue detail.
- [ ] Add review notes/comments model.
- [ ] Defer GitHub integration until core issue tracker is stable.

## Phase 15: production quality

- [ ] Add favicon/logo.
- [ ] Add loading skeletons.
- [ ] Add error boundaries.
- [ ] Add empty states for every module.
- [ ] Add permission checks to every Convex function.
- [ ] Add account/workspace deletion cleanup for new tables.
- [ ] Add seed data for all major modules.
- [ ] Add Docker production validation after schema changes.
- [ ] Keep `bun x ultracite check` clean.
- [ ] Keep Playwright suite deterministic and UI-only.

## Suggested immediate next tasks

1. Resolve SvelteKit vs Next.js.
2. Remove the unused worker package if it is still in the workspace.
3. Convert `/dashboard` into URL-driven workspace/project routes.
4. Build the kanban board with state columns.
5. Add drag-and-drop issue movement with Convex mutations.
6. Add multi-browser realtime Playwright coverage.
7. Add labels and assignees.
8. Add project settings for states and labels.
9. Add intake.
10. Add modules.
11. Add cycles.
12. Add pages.
