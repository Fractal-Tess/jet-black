<script lang="ts">
import ChevronLeft from "lucide-svelte/icons/chevron-left";
import ChevronRight from "lucide-svelte/icons/chevron-right";
import X from "lucide-svelte/icons/x";
import { onMount } from "svelte";

let {
  value = null,
  placeholder = "Pick a date",
  onChange,
  clearable = true,
  minDate,
  maxDate,
  disabled = false,
  disabledHint,
}: {
  value?: string | null;
  placeholder?: string;
  onChange: (date: string | null) => void;
  clearable?: boolean;
  minDate?: string | null;
  maxDate?: string | null;
  disabled?: boolean;
  /** Hover hint shown on dates disabled by minDate/maxDate. */
  disabledHint?: string;
} = $props();

let open = $state(false);
let panelTop = $state(0);
let panelLeft = $state(0);
let viewYear = $state(new Date().getFullYear());
let viewMonth = $state(new Date().getMonth());
let mounted = $state(false);

onMount(() => {
  mounted = true;
});

function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      if (node.parentNode) {
        node.parentNode.removeChild(node);
      }
    },
  };
}

$effect(() => {
  if (!open) {
    return;
  }
  // Open on the selected month, otherwise on a month that has selectable
  // days (today clamped into the min/max range).
  const now = new Date();
  let iso = value ?? toIso(now.getFullYear(), now.getMonth(), now.getDate());
  if (maxDate && iso > maxDate) {
    iso = maxDate;
  }
  if (minDate && iso < minDate) {
    iso = minDate;
  }
  const [y, m] = iso.split("-").map(Number);
  viewYear = y;
  viewMonth = m - 1;
});

const WEEKDAYS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
const MONTH_NAMES = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

function formatDate(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function toIso(year: number, month: number, day: number): string {
  return `${String(year).padStart(4, "0")}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function isToday(y: number, m: number, d: number): boolean {
  const n = new Date();
  return n.getFullYear() === y && n.getMonth() === m && n.getDate() === d;
}

function isSelected(y: number, m: number, d: number): boolean {
  return !!value && value === toIso(y, m, d);
}

function isPast(y: number, m: number, d: number): boolean {
  const n = new Date();
  return toIso(y, m, d) < toIso(n.getFullYear(), n.getMonth(), n.getDate());
}

function isDisabled(y: number, m: number, d: number): boolean {
  const iso = toIso(y, m, d);
  if (minDate && iso < minDate) {
    return true;
  }
  if (maxDate && iso > maxDate) {
    return true;
  }
  return false;
}

type CalendarDay = {
  day: number;
  isCurrentMonth: boolean;
  month: number;
  year: number;
};

const calendarDays: CalendarDay[][] = $derived.by(() => {
  const firstDay = new Date(viewYear, viewMonth, 1).getDay();
  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const daysInPrevMonth = new Date(viewYear, viewMonth, 0).getDate();
  const days: CalendarDay[] = [];

  for (let i = firstDay - 1; i >= 0; i--) {
    const pm = viewMonth === 0 ? 11 : viewMonth - 1;
    const py = viewMonth === 0 ? viewYear - 1 : viewYear;
    days.push({
      day: daysInPrevMonth - i,
      month: pm,
      year: py,
      isCurrentMonth: false,
    });
  }
  for (let d = 1; d <= daysInMonth; d++) {
    days.push({
      day: d,
      month: viewMonth,
      year: viewYear,
      isCurrentMonth: true,
    });
  }
  const remaining = 42 - days.length;
  const nm = viewMonth === 11 ? 0 : viewMonth + 1;
  const ny = viewMonth === 11 ? viewYear + 1 : viewYear;
  for (let d = 1; d <= remaining; d++) {
    days.push({ day: d, month: nm, year: ny, isCurrentMonth: false });
  }

  const weeks: CalendarDay[][] = [];
  for (let i = 0; i < days.length; i += 7) {
    weeks.push(days.slice(i, i + 7));
  }
  return weeks;
});

const displayText = $derived(value ? formatDate(value) : null);

function toggle(e: MouseEvent) {
  if (open) {
    open = false;
    return;
  }
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  // Clamp so the fixed panel stays fully inside the viewport.
  const PANEL_HEIGHT = 340;
  const PANEL_WIDTH = 270;
  panelTop = Math.max(
    8,
    Math.min(rect.bottom + 4, window.innerHeight - PANEL_HEIGHT)
  );
  panelLeft = Math.max(8, Math.min(rect.left, window.innerWidth - PANEL_WIDTH));
  open = true;
}

function close() {
  open = false;
}
</script>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") open = false; }} />

<div class="relative" data-dropdown>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span
    class="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1 text-sm transition-colors hover:bg-muted {disabled
      ? 'pointer-events-none opacity-50'
      : ''} {displayText ? 'text-foreground' : 'text-muted-foreground'}"
    onclick={toggle}
  >
    {displayText ?? placeholder}
    {#if clearable && displayText && !disabled}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span
        class="grid size-4 place-items-center rounded text-muted-foreground transition-colors hover:text-foreground"
        onclick={(e) => { e.stopPropagation(); onChange(null); }}
      >
        <X class="size-3" />
      </span>
    {/if}
  </span>
</div>

{#if open && mounted}
  <div use:portal>
    <button
      aria-label="Close date picker"
      class="fixed inset-0"
      onclick={close}
      style="z-index: 9998;"
      type="button"
    ></button>
    <div
      class="fixed w-[260px] rounded-xl border border-border bg-popover p-3 text-popover-foreground shadow-md"
      style="z-index: 9999; top: {panelTop}px; left: {panelLeft}px;"
    >
      <div class="flex items-center justify-between">
        <button
          aria-label="Previous month"
          class="grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          onclick={() => { if (viewMonth === 0) { viewMonth = 11; viewYear--; } else { viewMonth--; } }}
          type="button"
        >
          <ChevronLeft class="size-4" />
        </button>
        <span class="text-sm font-medium text-foreground">
          {MONTH_NAMES[viewMonth]} {viewYear}
        </span>
        <button
          aria-label="Next month"
          class="grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          onclick={() => { if (viewMonth === 11) { viewMonth = 0; viewYear++; } else { viewMonth++; } }}
          type="button"
        >
          <ChevronRight class="size-4" />
        </button>
      </div>

      <div class="mt-3 grid grid-cols-7">
        {#each WEEKDAYS as wd}
          <div class="text-center text-meta font-medium text-muted-foreground">{wd}</div>
        {/each}
      </div>

      <div class="mt-1 grid grid-cols-7">
        {#each calendarDays as week}
          {#each week as d}
            {@const sel = isSelected(d.year, d.month, d.day)}
            {@const tod = isToday(d.year, d.month, d.day)}
            {@const dis = isDisabled(d.year, d.month, d.day)}
            {@const past = isPast(d.year, d.month, d.day)}
            <button
              aria-disabled={dis}
              class="grid size-[34px] place-items-center rounded-md text-sm transition
                {sel
                  ? 'bg-primary font-semibold text-primary-foreground hover:bg-primary/80'
                  : tod
                    ? 'font-semibold text-primary hover:bg-muted'
                    : d.isCurrentMonth
                      ? past
                        ? 'text-muted-foreground/70 hover:bg-muted'
                        : 'text-foreground hover:bg-muted'
                      : past
                        ? 'text-muted-foreground/40'
                        : 'text-muted-foreground/50'}
                {dis ? 'cursor-not-allowed opacity-40 hover:bg-transparent' : ''}"
              onclick={() => {
                if (dis) {
                  return;
                }
                onChange(toIso(d.year, d.month, d.day));
                open = false;
              }}
              title={dis ? disabledHint : undefined}
              type="button"
            >
              {d.day}
            </button>
          {/each}
        {/each}
      </div>
    </div>
  </div>
{/if}
