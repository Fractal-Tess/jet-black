<script lang="ts">
import type { Project } from "$lib/components/issues/types";
import { projectModuleHref } from "$lib/routes";

let {
  project,
  workspaceSlug,
}: {
  project: Project;
  workspaceSlug: string;
} = $props();

const hasCover = $derived(Boolean(project.coverImageUrl));
const createdDate = $derived(
  project._creationTime
    ? new Date(project._creationTime).toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
      })
    : null
);

const projectInitial = $derived(project.name.trim()[0]?.toUpperCase() ?? "?");
</script>

<a
  href={projectModuleHref({
    module: "tickets",
    projectId: project._id,
    workspaceSlug,
  })}
  class="group/project-card flex flex-col overflow-hidden rounded-xl border border-border bg-card shadow-sm transition-all hover:border-input hover:shadow-md"
>
  <div class="relative h-[118px] w-full shrink-0 overflow-hidden">
    {#if hasCover}
      <img
        src={project.coverImageUrl}
        alt=""
        class="absolute inset-0 h-full w-full object-cover"
      />
    {/if}
    <div
      class="absolute inset-0 bg-gradient-to-t from-background/70 via-background/30 to-transparent"
    ></div>

    <div
      class="absolute bottom-3 left-3 right-3 z-[2] flex items-center gap-3"
    >
      <div
        class="grid size-9 shrink-0 place-items-center rounded-md bg-muted/80 backdrop-blur-sm"
      >
        {#if project.logoUrl}
          <img src={project.logoUrl} alt="" class="size-7 rounded object-contain" />
        {:else}
          <span class="grid size-7 place-items-center rounded bg-primary text-sm font-semibold text-primary-foreground">
            {projectInitial}
          </span>
        {/if}
      </div>
      <div class="min-w-0 flex-1">
        <h3 class="truncate text-sm font-semibold text-foreground">
          {project.name}
        </h3>
        <p class="text-meta font-medium text-foreground/70">
          {project.key}
        </p>
      </div>
    </div>
  </div>

  <div class="flex flex-1 flex-col justify-between px-4 py-3">
    <p class="line-clamp-2 text-sm leading-relaxed text-muted-foreground">
      {project.description?.trim() || `Created ${createdDate ?? "recently"}`}
    </p>
    <div class="mt-3 flex items-center justify-between">
      <span class="text-meta text-sidebar-muted-foreground">
        {createdDate ?? "New project"}
      </span>
      <span class="text-meta font-medium text-sidebar-muted-foreground">
        {project.key}
      </span>
    </div>
  </div>
</a>
