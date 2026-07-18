<script lang="ts">
import { Calendar as CalendarPrimitive } from "bits-ui";
import ChevronLeft from "lucide-svelte/icons/chevron-left";
import ChevronRight from "lucide-svelte/icons/chevron-right";
import { cn } from "../../utils.js";

type Props = CalendarPrimitive.RootProps;

let { class: className, ...restProps }: Props = $props();
</script>

<CalendarPrimitive.Root
  class={cn("p-3", className)}
  weekdayFormat="short"
  {...restProps}
>
  {#snippet children({ months, weekdays })}
    <CalendarPrimitive.Header class="relative flex w-full items-center justify-between">
      <CalendarPrimitive.PrevButton
        class="grid size-7 place-items-center rounded-md text-zinc-400 transition hover:bg-white/[0.08] hover:text-zinc-200"
      >
        <ChevronLeft class="size-4" />
      </CalendarPrimitive.PrevButton>
      <CalendarPrimitive.Heading class="text-sm font-medium text-zinc-200" />
      <CalendarPrimitive.NextButton
        class="grid size-7 place-items-center rounded-md text-zinc-400 transition hover:bg-white/[0.08] hover:text-zinc-200"
      >
        <ChevronRight class="size-4" />
      </CalendarPrimitive.NextButton>
    </CalendarPrimitive.Header>

    {#each months as month}
      <CalendarPrimitive.Grid class="mt-3 w-full border-collapse">
        <CalendarPrimitive.GridHead>
          <CalendarPrimitive.GridRow class="flex w-full">
            {#each weekdays as weekday}
              <CalendarPrimitive.HeadCell
                class="w-8 text-center text-[11px] font-medium text-zinc-500"
              >
                {weekday.slice(0, 2)}
              </CalendarPrimitive.HeadCell>
            {/each}
          </CalendarPrimitive.GridRow>
        </CalendarPrimitive.GridHead>
        <CalendarPrimitive.GridBody>
          {#each month.weeks as weekDates}
            <CalendarPrimitive.GridRow class="flex w-full">
              {#each weekDates as date}
                <CalendarPrimitive.Cell {date} month={month.value} class="p-0">
                  <CalendarPrimitive.Day
                    class={cn(
                      "grid size-8 place-items-center rounded-md text-sm transition",
                      "text-zinc-300 hover:bg-white/[0.08]",
                      "data-[today]:font-semibold data-[today]:text-amber-400",
                      "data-[selected]:bg-amber-400 data-[selected]:text-black data-[selected]:hover:bg-amber-300",
                      "data-[outside-month]:text-zinc-700 data-[outside-month]:hover:bg-transparent",
                      "data-[disabled]:text-zinc-700 data-[disabled]:hover:bg-transparent data-[disabled]:cursor-not-allowed"
                    )}
                  />
                </CalendarPrimitive.Cell>
              {/each}
            </CalendarPrimitive.GridRow>
          {/each}
        </CalendarPrimitive.GridBody>
      </CalendarPrimitive.Grid>
    {/each}
  {/snippet}
</CalendarPrimitive.Root>
