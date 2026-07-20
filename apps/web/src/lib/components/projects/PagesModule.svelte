<script lang="ts">
import { Card } from "@workspace/ui/components/card";
import type { ProjectPage } from "$lib/components/issues/types";
import PageEditor from "./PageEditor.svelte";
import PageForm from "./PageForm.svelte";

let {
  creating,
  onCreate,
  onUpdate,
  pages,
}: {
  creating: boolean;
  onCreate: (input: {
    content?: string;
    icon?: string;
    title: string;
  }) => Promise<void>;
  onUpdate: (
    pageId: ProjectPage["_id"],
    input: {
      content?: string;
      icon?: string | null;
      title?: string;
    }
  ) => Promise<void>;
  pages: ProjectPage[];
} = $props();

let selectedPageId = $state<ProjectPage["_id"] | undefined>();

const selectedPage = $derived(
  pages.find((page) => page._id === selectedPageId) ?? pages[0] ?? null
);
</script>

<section class="grid gap-5 xl:grid-cols-[360px_minmax(0,1fr)]">
  <div class="space-y-5">
    <PageForm {creating} {onCreate} />

    <Card>
      <div class="border-b border-border px-4 py-3">
      <p class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Docs
        </p>
        <h2 class="mt-1 text-lg font-semibold text-foreground">
          Project pages
        </h2>
      </div>

      <div class="divide-y divide-border">
        {#each pages as page (page._id)}
          <button
            class="flex w-full items-center gap-3 px-4 py-3 text-left transition hover:bg-accent {selectedPage?._id ===
            page._id
              ? 'bg-accent'
              : ''}"
            onclick={() => {
              selectedPageId = page._id;
            }}
            type="button"
          >
            <span class="grid size-8 place-items-center rounded-md bg-muted text-sm">
              {page.icon ?? "\u25a3"}
            </span>
            <span class="min-w-0">
              <span class="block truncate text-sm font-medium text-foreground">
                {page.title}
              </span>
              <span class="block truncate text-xs text-muted-foreground">
                {page.content || "Empty page"}
              </span>
            </span>
          </button>
        {:else}
          <p class="p-6 text-sm text-muted-foreground">No pages yet.</p>
        {/each}
      </div>
    </Card>
  </div>

  {#if selectedPage}
    <PageEditor page={selectedPage} {onUpdate} totalPages={pages.length} />
  {:else}
    <Card>
      <div class="grid min-h-96 place-items-center p-8 text-center">
        <div>
          <p class="text-sm font-medium text-secondary-foreground">
            No page selected.
          </p>
          <p class="mt-1 max-w-sm text-sm text-muted-foreground">
            Create a page to keep specs, decisions, and project docs next to
            the tickets.
          </p>
        </div>
      </div>
    </Card>
  {/if}
</section>
