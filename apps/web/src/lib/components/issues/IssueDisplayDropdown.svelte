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
  "h-7 w-32 cursor-pointer justify-between rounded-lg border-input bg-background px-2 text-xs text-foreground";
const selectContentClass =
  "border border-border bg-popover text-popover-foreground";

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
      class="flex h-8 cursor-pointer items-center gap-1.5 rounded-lg border border-input bg-background px-2.5 text-xs text-foreground transition-colors hover:bg-muted"
  >
    <SlidersHorizontal class="size-3.5" />
    Display
  </PopoverTrigger>
  <PopoverContent
    align="end"
    class="w-72 border border-border bg-popover p-0 text-popover-foreground"
  >
    <div class="space-y-4 p-4">
      <div class="flex items-center justify-between gap-4">
        <span class="text-xs text-muted-foreground">Group by</span>
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
        <span class="text-xs text-muted-foreground">Order by</span>
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

      <div class="space-y-2 border-t border-border pt-3">
        <span class="text-xs text-muted-foreground">Display properties</span>
        <div class="flex flex-wrap gap-1.5">
          {#each propertyKeys as property (property)}
            <button
              aria-pressed={displayOptions.properties[property]}
              class="rounded-md border px-2 py-1 text-meta transition {displayOptions
                .properties[property]
              ? 'border-primary/40 bg-primary/10 text-primary'
              : 'border-border text-muted-foreground hover:bg-muted hover:text-foreground'}"
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
