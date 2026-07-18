<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { untrack } from "svelte";
import AutoTab from "./AutoTab.svelte";
import SearchTab from "./SearchTab.svelte";
import UploadTab from "./UploadTab.svelte";

type ImageType = "cover" | "logo";

let {
  onSelect,
  projectId,
  projectName = "",
  selectedUrl = null,
  type,
}: {
  onSelect: (url: string | null) => void;
  projectId?: string;
  projectName?: string;
  selectedUrl?: string | null;
  type: ImageType;
} = $props();

const tabs = $derived(
  type === "logo"
    ? [
        { id: "auto" as const, label: "Auto" },
        { id: "search" as const, label: "Search" },
        { id: "upload" as const, label: "Upload" },
      ]
    : [
        { id: "search" as const, label: "Search" },
        { id: "upload" as const, label: "Upload" },
      ]
);

let activeTab = $state(untrack(() => (type === "logo" ? "auto" : "search")));

function handleRemove() {
  onSelect(null);
}
</script>

<div class="space-y-4">
  {#if selectedUrl}
    <div class="flex items-center gap-3">
      <img
        alt="Selected"
        class="h-16 w-16 rounded-lg border border-border object-cover"
        src={selectedUrl}
      />
      <Button variant="outline" class="h-8 px-3 text-xs" onclick={handleRemove}>
        Remove
      </Button>
    </div>
  {/if}

  <div class="flex border-b border-border">
    {#each tabs as tab (tab.id)}
      <button
        class="px-4 py-2 text-xs font-medium transition {activeTab === tab.id
          ? 'border-b-2 border-primary text-primary'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => {
          activeTab = tab.id;
        }}
        type="button"
      >
        {tab.label}
      </button>
    {/each}
  </div>

  {#if activeTab === "auto"}
    <AutoTab {projectName} onSelect={(url) => onSelect(url)} />
  {/if}

  {#if activeTab === "search"}
    <SearchTab onSelect={(url) => onSelect(url)} />
  {/if}

  {#if activeTab === "upload"}
    <UploadTab {projectId} {type} onSelect={(url) => onSelect(url)} />
  {/if}
</div>
