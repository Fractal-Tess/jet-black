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
    class="rounded-xl border border-red-400/20 bg-red-400/5 p-4 text-sm text-red-200"
  >
    Could not load your workspace: {error.message}
  </div>
{:else if loading}
  <div
    class="grid min-h-96 place-items-center rounded-xl border border-white/10 bg-[#151616] text-sm text-zinc-500"
  >
    Loading workspace data…
  </div>
{:else if !viewerData?.activeWorkspace}
  <div
    class="grid min-h-96 place-items-center rounded-xl border border-white/10 bg-[#151616] text-center"
  >
    <div>
      <p class="text-sm text-zinc-400">No workspace found.</p>
      <p class="mt-1 text-xs text-zinc-600">
        Set up your workspace to start collaborating.
      </p>
      <a
        class="mt-4 inline-flex h-9 items-center rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300"
        href="/onboarding"
      >
        Set up workspace
      </a>
    </div>
  </div>
{:else if !activeProject}
  <div
    class="grid min-h-96 place-items-center rounded-xl border border-white/10 bg-[#151616] text-center"
  >
    <div>
      <p class="text-sm text-zinc-400">
        Welcome to {viewerData?.activeWorkspace?.name ?? "your workspace"}.
      </p>
      <p class="mt-1 text-xs text-zinc-600">
        Create your first project to get started.
      </p>
      {#if viewerData?.activeWorkspace}
        <a
          class="mt-4 inline-flex h-9 items-center rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300"
          href={workspaceProjectsHref(viewerData.activeWorkspace.slug)}
        >
          Create project
        </a>
      {/if}
    </div>
  </div>
{/if}
