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
        class="grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      >
        <ChevronLeft class="size-4" />
      </CalendarPrimitive.PrevButton>
      <CalendarPrimitive.Heading class="text-sm font-medium text-foreground" />
      <CalendarPrimitive.NextButton
        class="grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
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
                class="w-8 text-center text-meta font-medium text-muted-foreground"
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
                      "text-foreground hover:bg-muted",
                      "data-[today]:font-semibold data-[today]:text-primary",
                      "data-[selected]:bg-primary data-[selected]:text-primary-foreground data-[selected]:hover:bg-primary/80",
                      "data-[outside-month]:text-muted-foreground/50 data-[outside-month]:hover:bg-transparent",
                      "data-[disabled]:cursor-not-allowed data-[disabled]:text-muted-foreground/50 data-[disabled]:hover:bg-transparent"
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
