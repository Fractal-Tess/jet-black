<script lang="ts">
import { api } from "@workspace/convex/api";
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { useAuth, useMutation, useQuery } from "convex-svelte";
import { goto } from "$app/navigation";
import type { ViewerData } from "$lib/components/issues/types";
import AppShell from "$lib/components/shell/AppShell.svelte";
import { projectModuleHref } from "$lib/routes";
import CreateProjectModal from "./CreateProjectModal.svelte";
import ProjectCard from "./ProjectCard.svelte";

let {
  user,
  workspaceSlug,
}: {
  user: {
    email: string;
    id: string;
    image?: string | null;
    name?: string | null;
  };
  workspaceSlug: string;
} = $props();

const auth = useAuth();
const viewer = useQuery(api.queries.workspaces.viewer, () =>
  auth.isAuthenticated ? { slug: workspaceSlug } : "skip"
);
const createProject = useMutation(api.mutations.projects.create);

const viewerData = $derived((viewer.data as ViewerData | undefined) ?? null);
const projects = $derived(viewerData?.projects ?? []);
const loading = $derived(viewer.isLoading);
const fallbackUser = $derived({
  email: user.email ?? "",
  image: user.image ?? null,
  name: user.name ?? user.email ?? "",
});

let createModalOpen = $state(false);
let creatingProject = $state(false);

async function onCreateProject(input: {
  description?: string;
  key: string;
  name: string;
}) {
  if (!viewerData?.activeWorkspace) {
    throw new Error("Workspace is still loading.");
  }

  creatingProject = true;

  try {
    const project = await createProject({
      description: input.description,
      key: input.key,
      name: input.name,
      workspaceId: viewerData.activeWorkspace._id,
    });

    if (project) {
      createModalOpen = false;
      await goto(
        projectModuleHref({
          module: "tickets",
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

function onSelectProject(projectId: string) {
  if (!workspaceSlug) {
    return;
  }
  goto(
    projectModuleHref({
      module: "tickets",
      projectId,
      workspaceSlug,
    })
  );
}
</script>

<svelte:head>
  <title>Projects &middot; Jet Black</title>
</svelte:head>

<AppShell
  activeWorkspace={viewerData?.activeWorkspace ?? null}
  activeWorkspaceSlug={workspaceSlug}
  connected={!viewer.isLoading && !viewer.error}
  {creatingProject}
  memberships={viewerData?.memberships ?? []}
  onCreateProject={onCreateProject}
  {onSelectProject}
  {projects}
  user={viewerData?.user ?? fallbackUser}
  workspaces={viewerData?.workspaces ?? []}
>
  <div class="mx-auto max-w-6xl px-5 py-8 sm:px-8">
    <div class="mb-8 flex items-center justify-between">
      <div>
        <h1 class="text-xl font-semibold tracking-tight text-foreground">
          Projects
        </h1>
        <p class="mt-1 text-sm text-muted-foreground">
          {projects.length} project{projects.length !== 1 ? "s" : ""}
        </p>
      </div>
      <Button onclick={() => (createModalOpen = true)}>
        <span class="text-base leading-none">+</span>
        Add project
      </Button>
    </div>

    {#if loading}
      <div class="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
        {#each Array(3) as _}
          <Card class="h-64 animate-pulse">
            <div class="h-[118px] rounded-t-lg bg-muted"></div>
            <div class="space-y-2 p-4">
              <div class="h-3 w-2/3 rounded bg-muted"></div>
              <div class="h-3 w-full rounded bg-muted/50"></div>
            </div>
          </Card>
        {/each}
      </div>
    {:else if projects.length === 0}
      <div
        class="flex flex-col items-center justify-center py-20 text-center"
      >
        <div
          class="mb-4 grid size-16 place-items-center rounded-full border border-dashed border-border bg-card"
        >
          <span class="text-2xl text-muted-foreground">&marker;</span>
        </div>
        <h2 class="text-base font-medium text-secondary-foreground">
          No projects yet
        </h2>
        <p class="mt-1 max-w-sm text-sm text-muted-foreground">
          Create your first project to start tracking issues, sprints, and
          modules.
        </p>
        <Button class="mt-6" onclick={() => (createModalOpen = true)}>
          <span class="text-base leading-none">+</span>
          Create your first project
        </Button>
      </div>
    {:else}
      <div class="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
        {#each projects as project (project._id)}
          <ProjectCard {project} {workspaceSlug} />
        {/each}
      </div>
    {/if}
  </div>
</AppShell>

{#if createModalOpen}
  <CreateProjectModal
    {creatingProject}
    {onCreateProject}
    onClose={() => (createModalOpen = false)}
    {workspaceSlug}
  />
{/if}
