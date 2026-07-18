<script lang="ts">
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@workspace/ui/components/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import SlidersHorizontal from "lucide-svelte/icons/sliders-horizontal";
import {
  DISPLAY_PROPERTY_LABELS,
  GROUP_BY_OPTIONS,
  type IssueDisplayOptions,
  type IssueDisplayProperty,
  ORDER_BY_OPTIONS,
} from "./display-options";

let {
  displayOptions,
  onChange,
}: {
  displayOptions: IssueDisplayOptions;
  onChange: (options: IssueDisplayOptions) => void;
} = $props();

const propertyKeys = Object.keys(
  DISPLAY_PROPERTY_LABELS
) as IssueDisplayProperty[];

const groupByLabel = $derived(
  GROUP_BY_OPTIONS.find((option) => option.value === displayOptions.groupBy)
    ?.label
);
const orderByLabel = $derived(
  ORDER_BY_OPTIONS.find((option) => option.value === displayOptions.orderBy)
    ?.label
);

const selectTriggerClass =
  "h-7 w-32 cursor-pointer justify-between rounded-md border-white/10 bg-[#101111] px-2 text-xs text-zinc-200";
const selectContentClass = "border border-white/10 bg-[#151616] text-zinc-200";

function toggleProperty(property: IssueDisplayProperty) {
  onChange({
    ...displayOptions,
    properties: {
      ...displayOptions.properties,
      [property]: !displayOptions.properties[property],
    },
  });
}
</script>

<Popover>
  <PopoverTrigger
    class="flex h-8 cursor-pointer items-center gap-1.5 rounded-md border border-white/10 bg-[#101111] px-2.5 text-xs text-zinc-300 transition hover:bg-white/[0.06]"
  >
    <SlidersHorizontal class="size-3.5" />
    Display
  </PopoverTrigger>
  <PopoverContent
    align="end"
    class="w-72 border border-white/10 bg-[#151616] p-0 text-zinc-200"
  >
    <div class="space-y-4 p-4">
      <div class="flex items-center justify-between gap-4">
        <span class="text-xs text-zinc-400">Group by</span>
        <Select
          onValueChange={(value) =>
            onChange({
              ...displayOptions,
              groupBy: value as IssueDisplayOptions["groupBy"],
            })}
          type="single"
          value={displayOptions.groupBy}
        >
          <SelectTrigger
            aria-label="Group by"
            class={selectTriggerClass}
            size="sm"
          >
            {groupByLabel}
          </SelectTrigger>
          <SelectContent class={selectContentClass}>
            {#each GROUP_BY_OPTIONS as option (option.value)}
              <SelectItem
                class="text-xs"
                label={option.label}
                value={option.value}
              />
            {/each}
          </SelectContent>
        </Select>
      </div>

      <div class="flex items-center justify-between gap-4">
        <span class="text-xs text-zinc-400">Order by</span>
        <Select
          onValueChange={(value) =>
            onChange({
              ...displayOptions,
              orderBy: value as IssueDisplayOptions["orderBy"],
            })}
          type="single"
          value={displayOptions.orderBy}
        >
          <SelectTrigger
            aria-label="Order by"
            class={selectTriggerClass}
            size="sm"
          >
            {orderByLabel}
          </SelectTrigger>
          <SelectContent class={selectContentClass}>
            {#each ORDER_BY_OPTIONS as option (option.value)}
              <SelectItem
                class="text-xs"
                label={option.label}
                value={option.value}
              />
            {/each}
          </SelectContent>
        </Select>
      </div>

      <div class="space-y-2 border-t border-white/[0.06] pt-3">
        <span class="text-xs text-zinc-400">Display properties</span>
        <div class="flex flex-wrap gap-1.5">
          {#each propertyKeys as property (property)}
            <button
              aria-pressed={displayOptions.properties[property]}
              class="rounded-md border px-2 py-1 text-[11px] transition {displayOptions
                .properties[property]
                ? 'border-amber-400/40 bg-amber-400/10 text-amber-300'
                : 'border-white/10 text-zinc-500 hover:bg-white/[0.06] hover:text-zinc-300'}"
              onclick={() => toggleProperty(property)}
              type="button"
            >
              {DISPLAY_PROPERTY_LABELS[property]}
            </button>
          {/each}
        </div>
      </div>
    </div>
  </PopoverContent>
</Popover>
