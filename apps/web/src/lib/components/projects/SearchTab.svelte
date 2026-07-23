<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Input } from "@workspace/ui/components/input";
import { env } from "$env/dynamic/public";

let {
  onSelect,
}: {
  onSelect: (url: string) => void;
} = $props();

const unsplashAccessKey = env.PUBLIC_UNSPLASH_ACCESS_KEY?.trim();

let searchQuery = $state("");
let debouncedQuery = $state("");
let searchResults = $state<
  Array<{ id: string; urls: { regular: string; small: string } }>
>([]);
let searchLoading = $state(false);
let searchError = $state("");
let selectedSearchUrl = $state<string | null>(null);
let searchTimeout: ReturnType<typeof setTimeout>;

$effect(() => {
  clearTimeout(searchTimeout);
  searchTimeout = setTimeout(() => {
    debouncedQuery = searchQuery;
  }, 300);
  return () => clearTimeout(searchTimeout);
});

async function searchUnsplash(query: string) {
  if (!(query.trim() && unsplashAccessKey)) {
    searchResults = [];
    return;
  }
  searchLoading = true;
  searchError = "";
  try {
    const response = await fetch(
      `https://api.unsplash.com/search/photos?query=${encodeURIComponent(query)}&per_page=12&client_id=${unsplashAccessKey}`
    );
    if (!response.ok) {
      throw new Error("Unsplash API error");
    }
    const data = await response.json();
    searchResults = data.results ?? [];
  } catch {
    searchError = "Could not load images";
    searchResults = [];
  } finally {
    searchLoading = false;
  }
}

$effect(() => {
  if (debouncedQuery.trim()) {
    searchUnsplash(debouncedQuery);
  } else {
    searchResults = [];
  }
});

function handleSelectSearch(url: string) {
  selectedSearchUrl = url;
}

function handleConfirmSearch() {
  if (selectedSearchUrl) {
    onSelect(selectedSearchUrl);
  }
}
</script>

<div class="space-y-3">
  <Input
    disabled={!unsplashAccessKey}
    placeholder="Search images\u2026"
    type="search"
    bind:value={searchQuery}
  />

  {#if !unsplashAccessKey}
    <div class="py-8 text-center text-sm text-muted-foreground">
      Unsplash image search is not configured.
    </div>
  {:else if searchLoading}
    <div class="py-8 text-center text-sm text-muted-foreground">Loading\u2026</div>
  {:else if searchError}
    <div class="py-8 text-center text-sm text-destructive">{searchError}</div>
  {:else if searchResults.length === 0 && debouncedQuery.trim()}
    <div class="py-8 text-center text-sm text-muted-foreground">No results</div>
  {:else if searchResults.length > 0}
    <div class="grid max-h-72 grid-cols-2 gap-2 overflow-y-auto">
      {#each searchResults as result (result.id)}
        <button
          class="group relative aspect-video overflow-hidden rounded-md border {selectedSearchUrl === result.urls.regular
            ? 'ring-2 ring-ring'
            : 'border-border'} transition hover:border-input"
          onclick={() => handleSelectSearch(result.urls.regular)}
          type="button"
        >
          <img
            alt=""
            class="h-full w-full object-cover transition group-hover:brightness-110"
            loading="lazy"
            src={result.urls.small}
          />
        </button>
      {/each}
    </div>

    {#if selectedSearchUrl}
      <Button class="w-full" onclick={handleConfirmSearch}>
        Use selected
      </Button>
    {/if}
  {/if}

    <p class="text-center text-meta text-muted-foreground">Photos by Unsplash</p>
</div>
