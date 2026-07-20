<script lang="ts">
import ArrowLeft from "lucide-svelte/icons/arrow-left";

let {
  backHref,
  title,
  entityName,
  entitySubtitle,
  entityInitials,
}: {
  backHref: string;
  title: string;
  entityName: string;
  entitySubtitle?: string;
  entityInitials?: string;
} = $props();

const initials = $derived(
  entityInitials ??
    entityName
      .trim()
      .split(/\s+/)
      .slice(0, 2)
      .map((w) => w[0])
      .join("")
      .toUpperCase()
);
</script>

<div class="mb-2">
  <a
    class="group flex items-center gap-1.5 rounded-md px-1 py-1 text-sm font-medium text-muted-foreground transition hover:text-foreground"
    href={backHref}
  >
    <ArrowLeft
      class="size-3.5 transition-transform group-hover:-translate-x-0.5"
    />
    <span class="truncate">{title}</span>
  </a>

  <div class="mt-2 flex items-center gap-2.5 px-1">
    <div class="grid size-8 shrink-0 place-items-center rounded-lg border border-primary/30 bg-primary text-meta font-bold text-primary-foreground">
      {initials}
    </div>
    <div class="min-w-0">
      <p class="truncate text-sm font-medium text-sidebar-foreground">
        {entityName}
      </p>
      {#if entitySubtitle}
        <p class="truncate text-meta text-sidebar-muted-foreground">
          {entitySubtitle}
        </p>
      {/if}
    </div>
  </div>
</div>
