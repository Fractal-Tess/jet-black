<script lang="ts">
import ArrowLeft from "lucide-svelte/icons/arrow-left";

let {
  backHref,
  title,
  entityName,
  entitySubtitle,
  entityColor,
  entityInitials,
}: {
  backHref: string;
  title: string;
  entityName: string;
  entitySubtitle?: string;
  entityColor?: string;
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
    class="group flex items-center gap-1.5 rounded-md px-1 py-1 text-[13px] font-medium text-muted-foreground transition hover:text-foreground"
    href={backHref}
  >
    <ArrowLeft
      class="size-3.5 transition-transform group-hover:-translate-x-0.5"
    />
    <span class="truncate">{title}</span>
  </a>

  <div class="mt-2 flex items-center gap-2.5 px-1">
    <div
      class="grid size-8 shrink-0 place-items-center rounded-md border border-border bg-muted text-[11px] font-bold text-sidebar-foreground"
      style:background-color={entityColor}
    >
      {initials}
    </div>
    <div class="min-w-0">
      <p class="truncate text-[13px] font-medium text-sidebar-foreground">
        {entityName}
      </p>
      {#if entitySubtitle}
        <p class="truncate text-[11px] text-sidebar-muted-foreground">
          {entitySubtitle}
        </p>
      {/if}
    </div>
  </div>
</div>
