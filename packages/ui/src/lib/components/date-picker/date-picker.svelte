<script lang="ts">
import type { DateValue } from "@internationalized/date";
import {
  CalendarDate,
  DateFormatter,
  getLocalTimeZone,
  parseDate,
  today,
} from "@internationalized/date";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import X from "lucide-svelte/icons/x";
import { cn } from "../../utils.js";
import { Calendar } from "../calendar/index.js";

let {
  value = null,
  placeholder = "Pick a date",
  onChange,
  clearable = true,
  minDate,
  maxDate,
  disabled = false,
  class: className,
}: {
  value?: string | null;
  placeholder?: string;
  onChange: (date: string | null) => void;
  clearable?: boolean;
  minDate?: string | null;
  maxDate?: string | null;
  disabled?: boolean;
  class?: string;
} = $props();

let open = $state(false);
let triggerEl: HTMLButtonElement | undefined = $state();
let panelEl: HTMLDivElement | undefined = $state();

const df = new DateFormatter("en-US", { dateStyle: "medium" });

const calendarValue: DateValue | undefined = $derived(
  value ? parseDate(value) : undefined
);

const minDateValue: DateValue | undefined = $derived(
  minDate ? parseDate(minDate) : undefined
);

const maxDateValue: DateValue | undefined = $derived(
  maxDate ? parseDate(maxDate) : undefined
);

const displayText = $derived(
  value ? df.format(parseDate(value).toDate(getLocalTimeZone())) : null
);

function handleSelect(date: DateValue | undefined) {
  if (!date) {
    return;
  }
  const iso = `${String(date.year).padStart(4, "0")}-${String(date.month).padStart(2, "0")}-${String(date.day).padStart(2, "0")}`;
  onChange(iso);
  open = false;
}

function handleClear(e: MouseEvent | KeyboardEvent) {
  e.stopPropagation();
  onChange(null);
  open = false;
}

function handleClickOutside(e: MouseEvent) {
  const target = e.target as Node;
  if (
    open &&
    triggerEl &&
    panelEl &&
    !triggerEl.contains(target) &&
    !panelEl.contains(target)
  ) {
    open = false;
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && open) {
    open = false;
  }
}
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKeydown} />

<div class={cn("relative", className)}>
  <button
    bind:this={triggerEl}
    class={cn(
      "flex items-center gap-2 rounded-lg px-2 py-1 text-sm transition-colors hover:bg-muted",
      disabled && "pointer-events-none opacity-50",
      displayText ? "text-foreground" : "text-muted-foreground"
    )}
    {disabled}
    onclick={() => (open = !open)}
    type="button"
  >
    <CalendarDays class="size-3.5 text-muted-foreground" />
    <span>{displayText ?? placeholder}</span>
    {#if clearable && displayText && !disabled}
      <span
        class="grid size-4 place-items-center rounded text-muted-foreground transition-colors hover:text-foreground"
        onclick={handleClear}
        onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handleClear(e); } }}
        role="button"
        tabindex="0"
      >
        <X class="size-3" />
      </span>
    {/if}
  </button>

  {#if open}
    <div
      bind:this={panelEl}
      class="absolute left-0 top-9 z-50 rounded-xl border border-border bg-popover text-popover-foreground shadow-md"
    >
      <Calendar
        type="single"
        value={calendarValue}
        onValueChange={handleSelect}
        minValue={minDateValue}
        maxValue={maxDateValue}
        placeholder={calendarValue ?? today(getLocalTimeZone())}
      />
    </div>
  {/if}
</div>
