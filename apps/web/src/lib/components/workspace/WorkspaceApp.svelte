<script lang="ts">
import { api } from "@workspace/convex/api";
import { useAuth, useMutation, useQuery } from "convex-svelte";
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";
import IssueDetail from "$lib/components/issues/IssueDetail.svelte";
import IssueStateSummary from "$lib/components/issues/IssueStateSummary.svelte";
import KanbanBoard from "$lib/components/issues/KanbanBoard.svelte";
import NewIssueForm from "$lib/components/issues/NewIssueForm.svelte";
import type {
  Issue,
  IssuePriority,
  IssueState,
  Project,
  ViewerData,
} from "$lib/components/issues/types";
import ProjectModulePlaceholder from "$lib/components/projects/ProjectModulePlaceholder.svelte";
import AppShell from "$lib/components/shell/AppShell.svelte";
import {
  issueHref,
  normalizeProjectModule,
  type ProjectModule,
  projectModuleHref,
} from "$lib/routes";

let {
  data,
  module = "issues",
  projectId,
  routeIssueId,
  workspaceSlug,
}: {
  data: {
    activeModule?: string;
    issueId?: string;
    module?: string;
    projectId?: string;
    user: {
      email: string;
      id: string;
      image?: string | null;
      name?: string | null;
    };
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
const routeIssue = $derived(routeIssueId ?? data.issueId);
const routeProjectId = $derived(projectId ?? data.projectId);
const routeWorkspaceSlug = $derived(workspaceSlug ?? data.workspaceSlug);
const auth = useAuth();
const ensurePersonalWorkspace = useMutation(
  api.mutations.workspaces.ensurePersonalWorkspace
);
const createIssue = useMutation(api.mutations.issues.create);
const updateIssue = useMutation(api.mutations.issues.update);
const createComment = useMutation(api.mutations.comments.create);
const createProject = useMutation(api.mutations.projects.create);

let selectedIssueId = $state<Issue["_id"] | undefined>();
let selectedProjectId = $state<Project["_id"] | undefined>();
let creating = $state(false);
let creatingProject = $state(false);
let ensuringWorkspace = $state(false);
let ensuredWorkspace = $state(false);
let checkingConvexToken = $state(false);
let convexTokenReady = $state(false);
let leavingAuthenticatedSession = $state(false);

const viewer = useQuery(api.queries.workspaces.viewer, () =>
  auth.isAuthenticated && convexTokenReady && !leavingAuthenticatedSession
    ? {}
    : "skip"
);

const viewerData = $derived((viewer.data as ViewerData | undefined) ?? null);
const activeProject = $derived(
  viewerData?.projects.find((project) => project._id === selectedProjectId) ??
    viewerData?.activeProject ??
    null
);
const issuesQuery = useQuery(api.queries.issues.listForProject, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  activeProject &&
  !leavingAuthenticatedSession
    ? { projectId: activeProject._id }
    : "skip"
);
const statesQuery = useQuery(api.queries.workspaces.statesForProject, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  activeProject &&
  !leavingAuthenticatedSession
    ? { projectId: activeProject._id }
    : "skip"
);
const issues = $derived((issuesQuery.data as Issue[] | undefined) ?? []);
const states = $derived((statesQuery.data as IssueState[] | undefined) ?? []);
const selectedIssue = $derived(
  issues.find((issue) => issue._id === selectedIssueId) ?? issues[0] ?? null
);
const commentsQuery = useQuery(api.queries.comments.listForIssue, () =>
  auth.isAuthenticated &&
  convexTokenReady &&
  selectedIssue &&
  !leavingAuthenticatedSession
    ? { issueId: selectedIssue._id }
    : "skip"
);
const comments = $derived(commentsQuery.data ?? []);
const fallbackUser = $derived({
  email: data.user.email,
  id: data.user.id,
  image: data.user.image ?? null,
  name: data.user.name ?? data.user.email,
});
const firstName = $derived(
  (viewerData?.user.name ?? fallbackUser.name).trim().split(/\s+/)[0] ?? "there"
);
const loadingRealtimeData = $derived(
  viewer.isLoading || issuesQuery.isLoading || statesQuery.isLoading
);
const realtimeError = $derived(
  Boolean(viewer.error || issuesQuery.error || statesQuery.error)
);
const connected = $derived(!(loadingRealtimeData || realtimeError));
const activeWorkspaceSlug = $derived(
  viewerData?.activeWorkspace?.slug ?? routeWorkspaceSlug
);

async function ensureWorkspace() {
  ensuringWorkspace = true;

  try {
    await ensurePersonalWorkspace({});
    ensuredWorkspace = true;
  } finally {
    ensuringWorkspace = false;
  }
}

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
  if (
    !auth.isAuthenticated ||
    auth.isLoading ||
    !convexTokenReady ||
    leavingAuthenticatedSession ||
    ensuredWorkspace ||
    ensuringWorkspace ||
    viewer.isLoading ||
    viewer.error ||
    !viewerData
  ) {
    return;
  }

  if (viewerData.activeWorkspace) {
    ensuredWorkspace = true;
    return;
  }

  ensureWorkspace();
});

$effect(() => {
  if (!auth.isAuthenticated) {
    convexTokenReady = false;
    return;
  }

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

async function handleCreateIssue(input: {
  description?: string;
  priority: IssuePriority;
  title: string;
}) {
  if (!activeProject) {
    return;
  }

  creating = true;

  try {
    const issueId = await createIssue({
      ...input,
      projectId: activeProject._id,
    });
    selectedIssueId = issueId;

    if (viewerData?.activeWorkspace) {
      await goto(
        issueHref({
          issueId,
          projectId: activeProject._id,
          workspaceSlug: viewerData.activeWorkspace.slug,
        })
      );
    }
  } finally {
    creating = false;
  }
}

async function handleUpdateIssue(input: {
  description?: string;
  priority?: IssuePriority;
  stateId?: IssueState["_id"];
  title?: string;
}) {
  if (!selectedIssue) {
    return;
  }

  await updateIssue({
    ...input,
    issueId: selectedIssue._id,
  });
}

async function handleAddComment(body: string) {
  if (!selectedIssue) {
    return;
  }

  await createComment({
    body,
    issueId: selectedIssue._id,
  });
}

async function handleMoveIssue(issue: Issue, stateId: IssueState["_id"]) {
  if (issue.stateId === stateId) {
    return;
  }

  await updateIssue({
    issueId: issue._id,
    stateId,
  });
}

async function handleCreateProject(input: { key: string; name: string }) {
  if (!viewerData?.activeWorkspace) {
    throw new Error("Workspace is still loading.");
  }

  creatingProject = true;

  try {
    const project = await createProject({
      key: input.key,
      name: input.name,
      workspaceId: viewerData.activeWorkspace._id,
    });

    if (project) {
      selectedIssueId = undefined;
      selectedProjectId = project._id;
      await goto(
        projectModuleHref({
          module: "issues",
          projectId: project._id,
          workspaceSlug: viewerData.activeWorkspace.slug,
        })
      );
      return true;
    }

    return false;
  } finally {
    creatingProject = false;
  }
}

async function handleSelectProject(nextProjectId: Project["_id"]) {
  selectedIssueId = undefined;
  selectedProjectId = nextProjectId;

  if (!viewerData?.activeWorkspace) {
    return;
  }

  await goto(
    projectModuleHref({
      module: "issues",
      projectId: nextProjectId,
      workspaceSlug: viewerData.activeWorkspace.slug,
    })
  );
}
</script>

{#if !auth.isAuthenticated && auth.isLoading}
  <div class="grid min-h-screen place-items-center bg-[#0d0e0e] text-zinc-500">
    Checking session…
  </div>
{:else}
  <AppShell
    {activeModule}
    {activeWorkspaceSlug}
    {connected}
    {creatingProject}
    onBeforeAuthExit={() => {
      convexTokenReady = false;
      leavingAuthenticatedSession = true;
    }}
    onCreateProject={handleCreateProject}
    onSelectProject={handleSelectProject}
    projects={viewerData?.projects ?? []}
    selectedProjectId={activeProject?._id}
    user={viewerData?.user ?? fallbackUser}
  >
    <div class="border-b border-white/10 px-5 py-4 sm:px-8">
      <div class="flex items-center gap-2 text-sm text-zinc-400">
        <span aria-hidden="true">⌂</span>
        <span>Home</span>
        <span class="text-zinc-700">/</span>
        <span>{activeProject?.name ?? "Issues"}</span>
        {#if activeModule !== "issues"}
          <span class="text-zinc-700">/</span>
          <span class="capitalize">{activeModule}</span>
        {/if}
      </div>
    </div>

    <div class="px-5 py-6 sm:px-8">
      {#if viewer.error}
        <div
          class="rounded-xl border border-red-400/20 bg-red-400/5 p-4 text-sm text-red-200"
        >
          Could not load your workspace: {viewer.error.message}
        </div>
      {:else if viewer.isLoading || ensuringWorkspace}
        <div
          class="grid min-h-96 place-items-center rounded-xl border border-white/10 bg-[#151616] text-sm text-zinc-500"
        >
          Preparing your workspace…
        </div>
      {:else if !activeProject}
        <div
          class="grid min-h-96 place-items-center rounded-xl border border-white/10 bg-[#151616] text-center"
        >
          <div>
            <p class="text-sm text-zinc-400">No project yet.</p>
            <button
              class="mt-4 h-9 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black"
              onclick={ensureWorkspace}
              type="button"
            >
              Create default workspace
            </button>
          </div>
        </div>
      {:else}
        <div class="mb-6 flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <p
              class="text-xs font-medium uppercase tracking-[0.18em] text-amber-400"
            >
              {viewerData?.activeWorkspace?.name ?? "Workspace"}
            </p>
            <h1 class="text-xl font-semibold tracking-tight text-zinc-100">
              Good to see you, {firstName}
            </h1>
            <h2 class="mt-1 text-2xl font-semibold tracking-tight text-zinc-100">
              {activeProject.name}
            </h2>
            <p class="mt-1 max-w-2xl text-sm text-zinc-500">
              {activeProject.description ??
                "Realtime issues, comments, and status changes."}
            </p>
          </div>
          <div class="flex gap-2 text-xs text-zinc-500">
            <span class="rounded-md border border-white/10 px-2 py-1">
              {issues.length} issues
            </span>
            <span class="rounded-md border border-white/10 px-2 py-1">
              {states.length} states
            </span>
          </div>
        </div>

        {#if activeModule !== "issues"}
          <ProjectModulePlaceholder
            module={activeModule}
            projectName={activeProject.name}
          />
        {:else}
          <div class="space-y-5">
            <IssueStateSummary {issues} {states} />

            <div class="grid gap-5 xl:grid-cols-[minmax(0,1fr)_420px]">
              <div class="space-y-5">
                <NewIssueForm
                  {creating}
                  onCreate={handleCreateIssue}
                  project={activeProject}
                />
                <KanbanBoard
                  {issues}
                  onMoveIssue={handleMoveIssue}
                  onSelect={async (issue) => {
                    selectedIssueId = issue._id;

                    if (viewerData?.activeWorkspace) {
                      await goto(
                        issueHref({
                          issueId: issue._id,
                          projectId: activeProject._id,
                          workspaceSlug: viewerData.activeWorkspace.slug,
                        })
                      );
                    }
                  }}
                  {selectedIssueId}
                  {states}
                />
              </div>

              <IssueDetail
                comments={comments}
                issue={selectedIssue}
                onAddComment={handleAddComment}
                onUpdateIssue={handleUpdateIssue}
                {states}
              />
            </div>
          </div>
        {/if}
      {/if}
    </div>
  </AppShell>
{/if}
