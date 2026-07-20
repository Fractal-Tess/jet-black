<script lang="ts">
import type { Project, ViewerData } from "$lib/components/issues/types";
import { workspaceProjectsHref } from "$lib/routes";

let {
  error,
  loading,
  viewerData,
  activeProject,
}: {
  error: Error | null | undefined;
  loading: boolean;
  viewerData: ViewerData | null;
  activeProject: Project | null;
} = $props();
</script>

{#if error}
  <div
      class="rounded-xl border border-destructive/20 bg-destructive/5 p-4 text-sm text-destructive"
  >
    Could not load your workspace: {error.message}
  </div>
{:else if loading}
  <div
      class="grid min-h-96 place-items-center rounded-xl border border-border bg-card text-sm text-muted-foreground shadow-sm"
  >
    Loading workspace data…
  </div>
{:else if !viewerData?.activeWorkspace}
  <div
      class="grid min-h-96 place-items-center rounded-xl border border-border bg-card text-center shadow-sm"
  >
    <div>
      <p class="text-sm text-muted-foreground">No workspace found.</p>
      <p class="mt-1 text-xs text-muted-foreground">
        Set up your workspace to start collaborating.
      </p>
      <a
        class="mt-4 inline-flex h-9 items-center rounded-lg bg-primary px-4 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/80"
        href="/onboarding"
      >
        Set up workspace
      </a>
    </div>
  </div>
{:else if !activeProject}
  <div
      class="grid min-h-96 place-items-center rounded-xl border border-border bg-card text-center shadow-sm"
  >
    <div>
      <p class="text-sm text-muted-foreground">
        Welcome to {viewerData?.activeWorkspace?.name ?? "your workspace"}.
      </p>
      <p class="mt-1 text-xs text-muted-foreground">
        Create your first project to get started.
      </p>
      {#if viewerData?.activeWorkspace}
        <a
        class="mt-4 inline-flex h-9 items-center rounded-lg bg-primary px-4 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/80"
          href={workspaceProjectsHref(viewerData.activeWorkspace.slug)}
        >
          Create project
        </a>
      {/if}
    </div>
  </div>
{/if}
