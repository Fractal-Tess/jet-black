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

const logoColor = $derived(project.color ?? logoColorFromName(project.name));
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

function logoColorFromName(name: string) {
  const palette = [
    "#f59e0b",
    "#3b82f6",
    "#22c55e",
    "#ef4444",
    "#8b5cf6",
    "#ec4899",
    "#06b6d4",
    "#f97316",
    "#84cc16",
    "#6366f1",
    "#14b8a6",
    "#e11d48",
    "#a855f7",
  ];
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = Math.imul(hash, 31) + name.charCodeAt(i);
  }
  return palette[Math.abs(hash) % palette.length];
}

function renderLogo() {
  if (project.logoUrl) {
    return project.logoUrl;
  }
  const letter = project.name.trim()[0]?.toUpperCase() ?? "?";
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 36 36"><rect width="36" height="36" rx="4" fill="${encodeURIComponent(logoColor)}"/><text x="18" y="23" text-anchor="middle" fill="white" font-size="18" font-weight="600" font-family="system-ui">${letter}</text></svg>`;
  return `data:image/svg+xml;charset=utf-8,${svg}`;
}
</script>

<a
  href={projectModuleHref({
    module: "tickets",
    projectId: project._id,
    workspaceSlug,
  })}
  class="group/project-card flex flex-col overflow-hidden rounded-lg border border-border bg-card transition hover:border-input hover:shadow-lg"
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
        <img
          src={renderLogo()}
          alt=""
          class="size-7 rounded object-contain"
        />
      </div>
      <div class="min-w-0 flex-1">
        <h3 class="truncate text-sm font-semibold text-foreground">
          {project.name}
        </h3>
        <p class="text-[11px] font-medium text-foreground/70">
          {project.key}
        </p>
      </div>
    </div>
  </div>

  <div class="flex flex-1 flex-col justify-between px-4 py-3">
    <p class="line-clamp-2 text-[13px] leading-relaxed text-muted-foreground">
      {project.description?.trim() || `Created ${createdDate ?? "recently"}`}
    </p>
    <div class="mt-3 flex items-center justify-between">
      <span class="text-[11px] text-sidebar-muted-foreground">
        {createdDate ?? "New project"}
      </span>
      <span class="text-[10px] font-medium text-sidebar-muted-foreground">
        {project.key}
      </span>
    </div>
  </div>
</a>
