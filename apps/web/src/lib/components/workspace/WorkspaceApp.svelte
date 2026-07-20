<script lang="ts">
import { api } from "@workspace/convex/api";
import {
  type UseQueryOptions,
  useAuth,
  useMutation,
  useQuery,
} from "convex-svelte";
import { authClient } from "$lib/auth-client";
import {
  defaultDisplayOptions,
  type IssueDisplayOptions,
} from "$lib/components/issues/display-options";
import IssuesHeader from "$lib/components/issues/IssuesHeader.svelte";
import type {
  IntakeIssue,
  Issue,
  IssueAttachment,
  IssueLabel,
  IssueLabelsQueryResult,
  IssueState,
  IssueStatesQueryResult,
  IssuesQueryResult,
  Project,
  ProjectModuleRecord,
  ProjectPage,
  Sprint,
  ViewerData,
  ViewerQueryResult,
  WorkspaceMember,
} from "$lib/components/issues/types";
import AppShell from "$lib/components/shell/AppShell.svelte";
import { isModuleEnabled } from "$lib/project-features";
import {
  normalizeProjectModule,
  type ProjectModule,
  type WorkspacePage,
} from "$lib/routes";
import WorkspaceAnalytics from "./WorkspaceAnalytics.svelte";
import WorkspaceCreateIssueModal from "./WorkspaceCreateIssueModal.svelte";
import WorkspaceDashboard from "./WorkspaceDashboard.svelte";
import WorkspaceProjectModuleOutlet from "./WorkspaceProjectModuleOutlet.svelte";
import WorkspaceStatusPanel from "./WorkspaceStatusPanel.svelte";
import type { WorkspaceActionDeps } from "./workspace-action-deps";
import { createWorkspaceIssueActions } from "./workspace-issue-actions.svelte";
import { createWorkspaceProjectActions } from "./workspace-project-actions.svelte";
import { createWorkspaceProjectModuleActions } from "./workspace-project-module-actions.svelte";

type PlaceholderProjectModule = Exclude<ProjectModule, "issues" | "tickets">;

let {
  data,
  module = "tickets",
  moduleId,
  projectId,
  routeIssueId,
  workspacePage = "home",
  workspaceSlug,
}: {
  data: {
    activeModule?: string;
    issueId?: string;
    module?: string;
    moduleId?: string;
    projectId?: string;
    user?: {
      email: string;
      id: string;
      image?: string | null;
      name?: string | null;
    };
    ssrIssues?: IssuesQueryResult;
    ssrLabels?: IssueLabelsQueryResult;
    ssrStates?: IssueStatesQueryResult;
    ssrViewer?: ViewerQueryResult;
    workspaceSlug?: string;
  };
  module?: string;
  moduleId?: string;
  projectId?: string;
  routeIssueId?: string;
  workspacePage?: WorkspacePage;
  workspaceSlug?: string;
} = $props();

const requestedModule: ProjectModule = $derived(
  normalizeProjectModule(module ?? data.module ?? data.activeModule)
);
const routeIssue = $derived(routeIssueId ?? data.issueId);
const routeProjectId = $derived(projectId ?? data.projectId);
const routeWorkspaceSlug = $derived(workspaceSlug ?? data.workspaceSlug);
const routeModuleId = $derived(moduleId ?? data.moduleId);
const isWorkspaceHome = $derived(
  workspacePage === "home" &&
    Boolean(routeWorkspaceSlug) &&
    !routeProjectId &&
    !routeIssue
);
const isWorkspaceAnalytics = $derived(
  workspacePage === "analytics" &&
    Boolean(routeWorkspaceSlug) &&
    !routeProjectId &&
    !routeIssue
);
const isProjectRoute = $derived(!(isWorkspaceHome || isWorkspaceAnalytics));
const activeWorkspacePage = $derived.by<WorkspacePage | null>(() => {
  if (isWorkspaceAnalytics) {
    return "analytics";
  }

  if (isWorkspaceHome) {
    return "home";
  }

  return null;
});
const auth = useAuth();

function placeholderProjectModule(
  value: ProjectModule
): PlaceholderProjectModule | null {
  if (value === "issues" || value === "tickets") {
    return null;
  }

  return value;
}

let analyticsProjectId = $state<Project["_id"] | undefined>();
let selectedIssueId = $state<Issue["_id"] | undefined>();
let selectedProjectId = $state<Project["_id"] | undefined>();
let issueCreateModalOpen = $state(false);
let issueCreateInitialStateId = $state<IssueState["_id"] | undefined>();
let checkingConvexToken = $state(false);
let convexTokenReady = $state(false);
let leavingAuthenticatedSession = $state(false);
let issueSearch = $state("");
let issueView = $state<"board" | "list">("board");
let displayOptions = $state<IssueDisplayOptions>(defaultDisplayOptions());

const viewer = useQuery(
  api.queries.workspaces.viewer,
  () =>
    (data.ssrViewer || (auth.isAuthenticated && convexTokenReady)) &&
    !leavingAuthenticatedSession
      ? { slug: routeWorkspaceSlug }
      : "skip",
  () =>
    data.ssrViewer
      ? ({
          initialData: data.ssrViewer,
          keepPreviousData: true,
        } satisfies UseQueryOptions<typeof api.queries.workspaces.viewer>)
      : {}
);

const viewerData = $derived(viewer.data ?? null);
const membersQuery = useQuery(api.queries.workspaces.membersForWorkspace, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  !leavingAuthenticatedSession &&
  viewerData?.activeWorkspace
    ? { workspaceId: viewerData.activeWorkspace._id }
    : "skip"
);
const workspaceMembers = $derived(membersQuery.data ?? []);
const dashboardQuery = useQuery(
  api.queries.dashboard.overviewForWorkspace,
  () =>
    auth.isAuthenticated &&
    convexTokenReady &&
    !leavingAuthenticatedSession &&
    isWorkspaceHome &&
    viewerData?.activeWorkspace
      ? { workspaceId: viewerData.activeWorkspace._id }
      : "skip"
);
const analyticsQuery = useQuery(
  api.queries.dashboard.analyticsForWorkspace,
  () =>
    auth.isAuthenticated &&
    convexTokenReady &&
    !leavingAuthenticatedSession &&
    isWorkspaceAnalytics &&
    viewerData?.activeWorkspace
      ? {
          projectId: analyticsProjectId,
          workspaceId: viewerData.activeWorkspace._id,
        }
      : "skip"
);
const activeProject = $derived(
  viewerData?.projects.find((project) => project._id === selectedProjectId) ??
    viewerData?.activeProject ??
    null
);
const activeModule: ProjectModule = $derived(
  isModuleEnabled(requestedModule, activeProject?.features)
    ? requestedModule
    : "tickets"
);
const activeTicketsModule = $derived(
  activeModule === "tickets" || activeModule === "issues"
);
const activePlaceholderModule = $derived(
  placeholderProjectModule(activeModule)
);

const issuesQuery = useQuery(
  api.queries.issues.listForProject,
  () =>
    (data.ssrIssues || (auth.isAuthenticated && convexTokenReady)) &&
    activeProject &&
    isProjectRoute &&
    !leavingAuthenticatedSession
      ? { projectId: activeProject._id }
      : "skip",
  () =>
    data.ssrIssues
      ? ({
          initialData: data.ssrIssues,
          keepPreviousData: true,
        } satisfies UseQueryOptions<typeof api.queries.issues.listForProject>)
      : {}
);

const statesQuery = useQuery(
  api.queries.workspaces.statesForProject,
  () =>
    (data.ssrStates || (auth.isAuthenticated && convexTokenReady)) &&
    activeProject &&
    isProjectRoute &&
    !leavingAuthenticatedSession
      ? { projectId: activeProject._id }
      : "skip",
  () =>
    data.ssrStates
      ? ({
          initialData: data.ssrStates,
          keepPreviousData: true,
        } satisfies UseQueryOptions<
          typeof api.queries.workspaces.statesForProject
        >)
      : {}
);

const labelsQuery = useQuery(
  api.queries.labels.listForProject,
  () =>
    (data.ssrLabels || (auth.isAuthenticated && convexTokenReady)) &&
    activeProject &&
    isProjectRoute &&
    !leavingAuthenticatedSession
      ? { projectId: activeProject._id }
      : "skip",
  () =>
    data.ssrLabels
      ? ({
          initialData: data.ssrLabels,
          keepPreviousData: true,
        } satisfies UseQueryOptions<typeof api.queries.labels.listForProject>)
      : {}
);
const intakeQuery = useQuery(api.queries.intake.listForProject, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  activeProject &&
  activeModule === "intake" &&
  !leavingAuthenticatedSession
    ? { projectId: activeProject._id }
    : "skip"
);
const sprintsQuery = useQuery(api.queries.sprints.listForProject, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  activeProject &&
  activeModule === "sprints" &&
  !leavingAuthenticatedSession
    ? { projectId: activeProject._id }
    : "skip"
);
// Not gated on the modules route: the issue detail panel offers a module
// picker wherever an issue is open.
const modulesQuery = useQuery(api.queries.modules.listForProject, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  activeProject &&
  !leavingAuthenticatedSession
    ? { projectId: activeProject._id }
    : "skip"
);
const archivedModulesQuery = useQuery(
  api.queries.modules.listArchivedForProject,
  () =>
    auth.isAuthenticated &&
    convexTokenReady &&
    activeProject &&
    activeModule === "modules" &&
    !leavingAuthenticatedSession
      ? { projectId: activeProject._id }
      : "skip"
);
const moduleDetailQuery = useQuery(api.queries.modules.get, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  routeModuleId &&
  activeModule === "modules" &&
  !leavingAuthenticatedSession
    ? { moduleId: routeModuleId as ProjectModuleRecord["_id"] }
    : "skip"
);
const pagesQuery = useQuery(api.queries.pages.listForProject, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  activeProject &&
  activeModule === "pages" &&
  !leavingAuthenticatedSession
    ? { projectId: activeProject._id }
    : "skip"
);
const issues = $derived(issuesQuery.data ?? []);
const intakeIssues = $derived(intakeQuery.data ?? []);
const modules = $derived(modulesQuery.data ?? []);
const archivedModules = $derived(archivedModulesQuery.data ?? []);
const moduleDetail = $derived(moduleDetailQuery.data ?? null);
const pages = $derived(pagesQuery.data ?? []);
const sprints = $derived(sprintsQuery.data ?? []);
const filteredIssues = $derived(
  issues.filter((issue) => {
    const search = issueSearch.trim().toLowerCase();

    if (!search) {
      return true;
    }

    return (
      issue.title.toLowerCase().includes(search) ||
      issue.identifier.toLowerCase().includes(search)
    );
  })
);
const states = $derived(statesQuery.data ?? []);
const labels = $derived(labelsQuery.data ?? []);
const selectedIssue = $derived(
  issues.find((issue) => issue._id === selectedIssueId) ?? issues[0] ?? null
);
const workspaceActionDeps: WorkspaceActionDeps = {
  getActiveProject: () => activeProject,
  getSelectedIssue: () => selectedIssue,
  getViewerData: () => viewerData,
  setSelectedIssueId: (issueId) => {
    selectedIssueId = issueId;
  },
  setSelectedProjectId: (nextProjectId) => {
    selectedProjectId = nextProjectId;
  },
};
const issueActions = createWorkspaceIssueActions(workspaceActionDeps);
const projectModuleActions =
  createWorkspaceProjectModuleActions(workspaceActionDeps);
const projectActions = createWorkspaceProjectActions(workspaceActionDeps);
const selectedSubIssues = $derived(
  selectedIssue
    ? issues.filter((issue) => issue.parentIssueId === selectedIssue._id)
    : []
);
const commentsQuery = useQuery(api.queries.comments.listForIssue, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  selectedIssue &&
  !leavingAuthenticatedSession
    ? { issueId: selectedIssue._id }
    : "skip"
);
const activitiesQuery = useQuery(api.queries.activities.listForIssue, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  selectedIssue &&
  !leavingAuthenticatedSession
    ? { issueId: selectedIssue._id }
    : "skip"
);
const attachmentsQuery = useQuery(api.queries.attachments.listForIssue, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  selectedIssue &&
  !leavingAuthenticatedSession
    ? { issueId: selectedIssue._id }
    : "skip"
);
const comments = $derived(commentsQuery.data ?? []);
const issueActivities = $derived(activitiesQuery.data ?? []);
const attachments = $derived(attachmentsQuery.data ?? []);
const fallbackUser = $derived({
  email: data.user?.email ?? "",
  image: data.user?.image ?? null,
  name: data.user?.name ?? data.user?.email ?? "",
});
const loadingRealtimeData = $derived(
  viewer.isLoading ||
    (isWorkspaceAnalytics && analyticsQuery.isLoading) ||
    issuesQuery.isLoading ||
    statesQuery.isLoading ||
    labelsQuery.isLoading ||
    modulesQuery.isLoading ||
    archivedModulesQuery.isLoading ||
    moduleDetailQuery.isLoading ||
    pagesQuery.isLoading ||
    intakeQuery.isLoading ||
    sprintsQuery.isLoading
);
const loadingWorkspaceData = $derived(
  viewer.isLoading ||
    (!viewerData && (!convexTokenReady || checkingConvexToken))
);
const realtimeError = $derived(
  Boolean(
    viewer.error ||
      (isWorkspaceAnalytics && analyticsQuery.error) ||
      issuesQuery.error ||
      statesQuery.error ||
      labelsQuery.error ||
      modulesQuery.error ||
      archivedModulesQuery.error ||
      moduleDetailQuery.error ||
      pagesQuery.error ||
      intakeQuery.error ||
      sprintsQuery.error
  )
);
const connected = $derived(!(loadingRealtimeData || realtimeError));
const activeWorkspaceSlug = $derived(
  viewerData?.activeWorkspace?.slug ?? routeWorkspaceSlug
);

async function refreshConvexTokenReady() {
  checkingConvexToken = true;

  try {
    const result = await authClient.convex.token({
      fetchOptions: { throw: false },
    });
    convexTokenReady = Boolean(result.data?.token);
  } finally {
    checkingConvexToken = false;
  }
}

$effect(() => {
  if (!auth.isAuthenticated) {
    convexTokenReady = false;
    return;
  }

  // When SSR-preloaded data exists, setupAuth already handles the token.
  // Skip the redundant token fetch to avoid re-render flashes during hydration.

  if (checkingConvexToken || convexTokenReady) {
    return;
  }

  refreshConvexTokenReady();
});

const claimInvites = useMutation(api.mutations.workspaceMembers.claimInvites);
let invitesClaimed = false;

$effect(() => {
  if (
    auth.isAuthenticated &&
    convexTokenReady &&
    !leavingAuthenticatedSession &&
    !invitesClaimed
  ) {
    invitesClaimed = true;
    claimInvites({}).catch(() => {
      // Ignore claim failures; the user still sees their existing workspaces.
    });
  }
});

$effect(() => {
  selectedProjectId = routeProjectId as Project["_id"] | undefined;
});

$effect(() => {
  if (
    analyticsProjectId &&
    !viewerData?.projects.some((project) => project._id === analyticsProjectId)
  ) {
    analyticsProjectId = undefined;
  }
});

$effect(() => {
  selectedIssueId = routeIssueId as Issue["_id"] | undefined;
});

$effect(() => {
  if (!selectedProjectId && viewerData?.activeProject) {
    selectedProjectId = viewerData.activeProject._id;
  }
});

$effect(() => {
  if (!selectedIssueId && issues[0]) {
    selectedIssueId = issues[0]._id;
  }
});

$effect(() => {
  if (
    selectedIssueId &&
    !issues.some((issue) => issue._id === selectedIssueId)
  ) {
    selectedIssueId = issues[0]?._id;
  }
});
</script>

{#if !viewer.data && !auth.isAuthenticated && auth.isLoading}
  <div class="grid min-h-screen place-items-center bg-background text-muted-foreground">
    Checking session…
  </div>
{:else}
  <AppShell
    {activeModule}
    {activeWorkspacePage}
    activeWorkspace={viewerData?.activeWorkspace ?? null}
    {activeWorkspaceSlug}
    {connected}
    creatingProject={projectActions.creatingProject}
    memberships={viewerData?.memberships ?? []}
    onBeforeAuthExit={() => {
      convexTokenReady = false;
      leavingAuthenticatedSession = true;
    }}
    onCreateProject={projectActions.onCreateProject}
    onSelectProject={projectActions.onSelectProject}
    projects={viewerData?.projects ?? []}
    selectedProjectId={activeProject?._id}
    user={viewerData?.user ?? fallbackUser}
    workspaces={viewerData?.workspaces ?? []}
  >
    {#if activeProject && activeWorkspaceSlug && isProjectRoute && activeTicketsModule}
      <IssuesHeader
        {activeProject}
        issueCount={issues.length}
        {displayOptions}
        {issueView}
        onAddWorkItem={() => (issueCreateModalOpen = true)}
        onDisplayOptionsChange={(options) => (displayOptions = options)}
        onIssueViewChange={(view) => (issueView = view)}
        onSearchChange={(query) => (issueSearch = query)}
        searchQuery={issueSearch}
        workspaceSlug={activeWorkspaceSlug}
      />
    {/if}

    <div class={activeModule === "modules" && isProjectRoute ? "" : "px-5 py-4 sm:px-8"}>
      {#if viewer.error || loadingWorkspaceData || !viewerData?.activeWorkspace}
        <WorkspaceStatusPanel
          error={viewer.error}
          loading={loadingWorkspaceData}
          {viewerData}
          {activeProject}
        />
      {:else if isWorkspaceAnalytics}
        <WorkspaceAnalytics
          analytics={analyticsQuery.data ?? null}
          loading={analyticsQuery.isLoading}
          onProjectChange={(projectId) => (analyticsProjectId = projectId)}
          projects={viewerData.projects}
          selectedProjectId={analyticsProjectId}
        />
      {:else if isWorkspaceHome}
        <WorkspaceDashboard
          loading={dashboardQuery.isLoading}
          members={workspaceMembers}
          overview={dashboardQuery.data ?? null}
          user={viewerData.user}
          workspaceName={viewerData.activeWorkspace.name}
          workspaceSlug={viewerData.activeWorkspace.slug}
        />
      {:else if !activeProject}
        <WorkspaceStatusPanel
          error={viewer.error}
          loading={loadingWorkspaceData}
          {viewerData}
          {activeProject}
        />
      {:else}
        <WorkspaceProjectModuleOutlet
          {activeModule}
          {activePlaceholderModule}
          {activeProject}
          workspaceSlug={viewerData?.activeWorkspace?.slug ?? ""}
          creatingIntake={projectModuleActions.creatingIntake}
          creatingSprint={projectModuleActions.creatingSprint}
          creatingModule={projectModuleActions.creatingModule}
          creatingPage={projectModuleActions.creatingPage}
          {intakeIssues}
          {filteredIssues}
          {issues}
          {sprints}
          {modules}
           {archivedModules}
           {moduleDetail}
           moduleDetailLoading={moduleDetailQuery.isLoading}
           routeModuleId={routeModuleId ?? undefined}
          {pages}
          {attachments}
          activities={issueActivities}
          comments={comments}
          {displayOptions}
          {issueView}
          {labels}
          workspaceMembers={workspaceMembers}
          {selectedIssueId}
          selectedSubIssues={selectedSubIssues}
          {states}
          onSelectIssue={(issueId) => (selectedIssueId = issueId)}
          onAcceptIntakeIssue={projectModuleActions.onAcceptIntakeIssue}
          onCreateIntakeIssue={projectModuleActions.onCreateIntakeIssue}
          onDeclineIntakeIssue={projectModuleActions.onDeclineIntakeIssue}
          onCreateSprint={projectModuleActions.onCreateSprint}
          onAssignIssueToSprint={projectModuleActions.onAssignIssueToSprint}
          onCreateModule={projectModuleActions.onCreateModule}
          onUpdateModule={projectModuleActions.onUpdateModule}
          onArchiveModule={projectModuleActions.onArchiveModule}
          onRestoreModule={projectModuleActions.onRestoreModule}
          onDeleteModule={projectModuleActions.onDeleteModule}
          onAssignIssueToModule={projectModuleActions.onAssignIssueToModule}
          onRemoveIssueFromModule={projectModuleActions.onRemoveIssueFromModule}
          onCreateModuleIssue={projectModuleActions.onCreateModuleIssue}
          onAddModuleLink={projectModuleActions.onAddModuleLink}
          onUpdateModuleLink={projectModuleActions.onUpdateModuleLink}
          onRemoveModuleLink={projectModuleActions.onRemoveModuleLink}
          onCreatePage={projectModuleActions.onCreatePage}
          onUpdatePage={projectModuleActions.onUpdatePage}
          onAddAttachment={issueActions.onAddAttachment}
          onAddComment={issueActions.onAddComment}
          onUpdateComment={issueActions.onUpdateComment}
          onDeleteComment={issueActions.onDeleteComment}
          onCreateModuleForIssue={issueActions.onCreateModuleForIssue}
          onCreateLabel={issueActions.onCreateLabel}
          onCreateSubIssue={issueActions.onCreateSubIssue}
          onArchiveIssue={issueActions.onArchiveIssue}
          onMoveIssue={issueActions.onMoveIssue}
          onOpenCreateIssue={(stateId) => {
            issueCreateInitialStateId = stateId;
            issueCreateModalOpen = true;
          }}
          onQuickCreateIssue={issueActions.onQuickCreateIssue}
          onReorderIssue={issueActions.onReorderIssue}
          onToggleLabel={issueActions.onToggleLabel}
          onUpdateIssue={issueActions.onUpdateIssue}
          onUpdateIssueFor={issueActions.onUpdateIssueFor}
        />
      {/if}
    </div>
  </AppShell>

  <WorkspaceCreateIssueModal
    open={issueCreateModalOpen}
    {activeProject}
    creating={issueActions.creating}
    initialStateId={issueCreateInitialStateId}
    members={workspaceMembers}
    onClose={() => {
      issueCreateModalOpen = false;
      issueCreateInitialStateId = undefined;
    }}
    onCreate={issueActions.onCreateIssue}
    {states}
  />
{/if}
