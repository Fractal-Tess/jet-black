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
}: {
  value?: string | null;
  placeholder?: string;
  onChange: (date: string | null) => void;
  clearable?: boolean;
  minDate?: string | null;
  maxDate?: string | null;
  disabled?: boolean;
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
  if (open && value) {
    const [y, m] = value.split("-").map(Number);
    viewYear = y;
    viewMonth = m - 1;
  }
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
  panelTop = rect.bottom + 4;
  panelLeft = rect.left;
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
    class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1 text-sm transition hover:bg-white/[0.06] {disabled
      ? 'pointer-events-none opacity-50'
      : ''} {displayText ? 'text-zinc-300' : 'text-zinc-500'}"
    onclick={toggle}
  >
    {displayText ?? placeholder}
    {#if clearable && displayText && !disabled}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span
        class="grid size-4 place-items-center rounded text-zinc-600 transition hover:text-zinc-300"
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
      class="fixed w-[260px] rounded-lg border border-white/10 bg-[#1a1b1b] p-3 shadow-xl"
      style="z-index: 9999; top: {panelTop}px; left: {panelLeft}px;"
    >
      <div class="flex items-center justify-between">
        <button
          class="grid size-7 place-items-center rounded-md text-zinc-400 transition hover:bg-white/[0.08] hover:text-zinc-200"
          onclick={() => { if (viewMonth === 0) { viewMonth = 11; viewYear--; } else { viewMonth--; } }}
          type="button"
        >
          <ChevronLeft class="size-4" />
        </button>
        <span class="text-sm font-medium text-zinc-200">
          {MONTH_NAMES[viewMonth]} {viewYear}
        </span>
        <button
          class="grid size-7 place-items-center rounded-md text-zinc-400 transition hover:bg-white/[0.08] hover:text-zinc-200"
          onclick={() => { if (viewMonth === 11) { viewMonth = 0; viewYear++; } else { viewMonth++; } }}
          type="button"
        >
          <ChevronRight class="size-4" />
        </button>
      </div>

      <div class="mt-3 grid grid-cols-7">
        {#each WEEKDAYS as wd}
          <div class="text-center text-[11px] font-medium text-zinc-500">{wd}</div>
        {/each}
      </div>

      <div class="mt-1 grid grid-cols-7">
        {#each calendarDays as week}
          {#each week as d}
            {@const sel = isSelected(d.year, d.month, d.day)}
            {@const tod = isToday(d.year, d.month, d.day)}
            {@const dis = isDisabled(d.year, d.month, d.day)}
            <button
              class="grid size-[34px] place-items-center rounded-md text-sm transition
                {sel
                  ? 'bg-amber-400 font-semibold text-black hover:bg-amber-300'
                  : tod
                    ? 'font-semibold text-amber-400 hover:bg-white/[0.08]'
                    : d.isCurrentMonth
                      ? 'text-zinc-300 hover:bg-white/[0.08]'
                      : 'text-zinc-700'}
                {dis ? 'cursor-not-allowed opacity-40 hover:bg-transparent' : ''}"
              disabled={dis}
              onclick={() => { onChange(toIso(d.year, d.month, d.day)); open = false; }}
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
