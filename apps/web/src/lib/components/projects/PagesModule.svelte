<script lang="ts">
import type { ProjectPage } from "$lib/components/issues/types";

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

let content = $state("");
let draftContent = $state("");
let draftIcon = $state("");
let draftTitle = $state("");
let icon = $state("▣");
let selectedPageId = $state<ProjectPage["_id"] | undefined>();
let title = $state("");

const selectedPage = $derived(
  pages.find((page) => page._id === selectedPageId) ?? pages[0] ?? null
);

$effect(() => {
  if (!selectedPage) {
    draftContent = "";
    draftIcon = "";
    draftTitle = "";
    return;
  }

  draftContent = selectedPage.content;
  draftIcon = selectedPage.icon ?? "";
  draftTitle = selectedPage.title;
});

async function createPage() {
  const nextTitle = title.trim();

  if (!nextTitle) {
    return;
  }

  await onCreate({
    content: content.trim(),
    icon: icon.trim() || undefined,
    title: nextTitle,
  });
  content = "";
  icon = "▣";
  title = "";
}

async function saveSelectedPage() {
  if (!selectedPage) {
    return;
  }

  await onUpdate(selectedPage._id, {
    content: draftContent,
    icon: draftIcon.trim() || null,
    title: draftTitle,
  });
}
</script>

<section class="grid gap-5 xl:grid-cols-[360px_minmax(0,1fr)]">
  <div class="space-y-5">
    <div class="rounded-xl border border-white/10 bg-[#151616]">
      <div class="border-b border-white/[0.06] px-4 py-3">
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Pages
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">New page</h2>
      </div>

      <form
        class="grid gap-3 p-4"
        onsubmit={(event) => {
          event.preventDefault();
          createPage();
        }}
      >
        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">Page title</span>
          <input
            bind:value={title}
            class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="Spec, decision, launch plan"
          />
        </label>

        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">Page icon</span>
          <input
            bind:value={icon}
            class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="▣"
          />
        </label>

        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">Page content</span>
          <textarea
            bind:value={content}
            class="min-h-32 w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="Write notes, specs, decisions, or project context."
          ></textarea>
        </label>

        <button
          class="h-9 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
          disabled={creating || !title.trim()}
          type="submit"
        >
          {creating ? "Creating…" : "Create page"}
        </button>
      </form>
    </div>

    <div class="rounded-xl border border-white/10 bg-[#151616]">
      <div class="border-b border-white/[0.06] px-4 py-3">
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Docs
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">
          Project pages
        </h2>
      </div>

      <div class="divide-y divide-white/[0.06]">
        {#each pages as page (page._id)}
          <button
            class="flex w-full items-center gap-3 px-4 py-3 text-left transition hover:bg-white/[0.03] {selectedPage?._id ===
            page._id
              ? 'bg-white/[0.05]'
              : ''}"
            onclick={() => {
              selectedPageId = page._id;
            }}
            type="button"
          >
            <span class="grid size-8 place-items-center rounded-md bg-white/[0.04] text-sm">
              {page.icon ?? "▣"}
            </span>
            <span class="min-w-0">
              <span class="block truncate text-sm font-medium text-zinc-200">
                {page.title}
              </span>
              <span class="block truncate text-xs text-zinc-600">
                {page.content || "Empty page"}
              </span>
            </span>
          </button>
        {:else}
          <p class="p-6 text-sm text-zinc-600">No pages yet.</p>
        {/each}
      </div>
    </div>
  </div>

  <div class="rounded-xl border border-white/10 bg-[#151616]">
    <div
      class="flex items-center justify-between border-b border-white/[0.06] px-4 py-3"
    >
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
          Editor
        </p>
        <h2 class="mt-1 text-lg font-semibold text-zinc-100">
          {selectedPage?.title ?? "Select a page"}
        </h2>
      </div>
      <span class="rounded-md border border-white/10 px-2 py-1 text-xs text-zinc-500">
        {pages.length} total
      </span>
    </div>

    {#if selectedPage}
      <div class="grid gap-4 p-4">
        <div class="grid gap-3 sm:grid-cols-[72px_minmax(0,1fr)]">
          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">Icon</span>
            <input
              bind:value={draftIcon}
              class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition focus:border-amber-400/60"
            />
          </label>
          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">Title</span>
            <input
              bind:value={draftTitle}
              class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition focus:border-amber-400/60"
            />
          </label>
        </div>

        <label class="block">
          <span class="mb-1 block text-xs text-zinc-500">Content</span>
          <textarea
            bind:value={draftContent}
            class="min-h-[360px] w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-4 py-3 font-mono text-sm leading-6 text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
          ></textarea>
        </label>

        <button
          class="h-9 justify-self-start rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
          disabled={!draftTitle.trim()}
          onclick={saveSelectedPage}
          type="button"
        >
          Save page
        </button>
      </div>
    {:else}
      <div class="grid min-h-96 place-items-center p-8 text-center">
        <div>
          <p class="text-sm font-medium text-zinc-300">
            No page selected.
          </p>
          <p class="mt-1 max-w-sm text-sm text-zinc-600">
            Create a page to keep specs, decisions, and project docs next to
            the tickets.
          </p>
        </div>
      </div>
    {/if}
  </div>
</section>
