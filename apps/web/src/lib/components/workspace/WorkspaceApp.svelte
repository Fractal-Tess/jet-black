<script lang="ts">
import { api } from "@workspace/convex/api";
import { type UseQueryOptions, useAuth, useQuery } from "convex-svelte";
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
import { normalizeProjectModule, type ProjectModule } from "$lib/routes";
import WorkspaceCreateIssueModal from "./WorkspaceCreateIssueModal.svelte";
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
  projectId,
  routeIssueId,
  workspaceSlug,
}: {
  data: {
    activeModule?: string;
    issueId?: string;
    module?: string;
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
  projectId?: string;
  routeIssueId?: string;
  workspaceSlug?: string;
} = $props();

const activeModule: ProjectModule = $derived(
  normalizeProjectModule(module ?? data.module ?? data.activeModule)
);
const activeTicketsModule = $derived(
  activeModule === "tickets" || activeModule === "issues"
);
const activePlaceholderModule = $derived(
  placeholderProjectModule(activeModule)
);
const routeIssue = $derived(routeIssueId ?? data.issueId);
const routeProjectId = $derived(projectId ?? data.projectId);
const routeWorkspaceSlug = $derived(workspaceSlug ?? data.workspaceSlug);
const auth = useAuth();

function placeholderProjectModule(
  value: ProjectModule
): PlaceholderProjectModule | null {
  if (value === "issues" || value === "tickets") {
    return null;
  }

  return value;
}

let selectedIssueId = $state<Issue["_id"] | undefined>();
let selectedProjectId = $state<Project["_id"] | undefined>();
let issueCreateModalOpen = $state(false);
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
const activeProject = $derived(
  viewerData?.projects.find((project) => project._id === selectedProjectId) ??
    viewerData?.activeProject ??
    null
);

const issuesQuery = useQuery(
  api.queries.issues.listForProject,
  () =>
    (data.ssrIssues || (auth.isAuthenticated && convexTokenReady)) &&
    activeProject &&
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
const modulesQuery = useQuery(api.queries.modules.listForProject, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  activeProject &&
  activeModule === "modules" &&
  !leavingAuthenticatedSession
    ? { projectId: activeProject._id }
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
const attachmentsQuery = useQuery(api.queries.attachments.listForIssue, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  selectedIssue &&
  !leavingAuthenticatedSession
    ? { issueId: selectedIssue._id }
    : "skip"
);
const comments = $derived(commentsQuery.data ?? []);
const attachments = $derived(attachmentsQuery.data ?? []);
const fallbackUser = $derived({
  email: data.user?.email ?? "",
  image: data.user?.image ?? null,
  name: data.user?.name ?? data.user?.email ?? "",
});
const loadingRealtimeData = $derived(
  viewer.isLoading ||
    issuesQuery.isLoading ||
    statesQuery.isLoading ||
    labelsQuery.isLoading ||
    modulesQuery.isLoading ||
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
      issuesQuery.error ||
      statesQuery.error ||
      labelsQuery.error ||
      modulesQuery.error ||
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

$effect(() => {
  selectedProjectId = routeProjectId as Project["_id"] | undefined;
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
    {#if activeProject && activeWorkspaceSlug}
      <IssuesHeader
        {activeProject}
        issueCount={issues.length}
        {displayOptions}
        {issueView}
        onAddWorkItem={() => (issueCreateModalOpen = true)}
        onDisplayOptionsChange={(options) => (displayOptions = options)}
        onIssueViewChange={(view) => (issueView = view)}
        workspaceSlug={activeWorkspaceSlug}
      />
    {/if}

    <div class="px-5 py-4 sm:px-8">
      {#if viewer.error || loadingWorkspaceData || !viewerData?.activeWorkspace || !activeProject}
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
          {sprints}
          {modules}
          {pages}
          {attachments}
          comments={comments}
          {displayOptions}
          {issueView}
          {labels}
          workspaceMembers={workspaceMembers}
          {selectedIssueId}
          selectedSubIssues={selectedSubIssues}
          {states}
          onAcceptIntakeIssue={projectModuleActions.onAcceptIntakeIssue}
          onCreateIntakeIssue={projectModuleActions.onCreateIntakeIssue}
          onDeclineIntakeIssue={projectModuleActions.onDeclineIntakeIssue}
          onCreateSprint={projectModuleActions.onCreateSprint}
          onAssignIssueToSprint={projectModuleActions.onAssignIssueToSprint}
          onCreateModule={projectModuleActions.onCreateModule}
          onAssignIssueToModule={projectModuleActions.onAssignIssueToModule}
          onCreatePage={projectModuleActions.onCreatePage}
          onUpdatePage={projectModuleActions.onUpdatePage}
          onAddAttachment={issueActions.onAddAttachment}
          onAddComment={issueActions.onAddComment}
          onCreateLabel={issueActions.onCreateLabel}
          onCreateSubIssue={issueActions.onCreateSubIssue}
          onArchiveIssue={issueActions.onArchiveIssue}
          onMoveIssue={issueActions.onMoveIssue}
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
    onClose={() => (issueCreateModalOpen = false)}
    onCreate={issueActions.onCreateIssue}
  />
{/if}
