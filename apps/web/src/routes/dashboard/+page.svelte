<script lang="ts">
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";

let { data } = $props();

let signingOut = $state(false);
let sidebarOpen = $state(false);

const firstName = $derived(data.user.name?.trim().split(/\s+/)[0] ?? "there");
const initials = $derived(
  data.user.name
    ?.trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part: string) => part[0])
    .join("")
    .toUpperCase() ?? data.user.email.slice(0, 2).toUpperCase()
);

async function signOut() {
  signingOut = true;

  try {
    await authClient.signOut();
    await goto("/login");
  } finally {
    signingOut = false;
  }
}
</script>

<svelte:head>
  <title>Home · Jet Black</title>
</svelte:head>

<div class="min-h-screen bg-[#0d0e0e] text-zinc-200">
  <header
    class="fixed inset-x-0 top-0 z-30 flex h-12 items-center border-b border-white/10 bg-[#0d0e0e]"
  >
    <div
      class="flex h-full w-[248px] shrink-0 items-center border-r border-white/10 px-3"
    >
      <button
        aria-label="Open navigation"
        class="mr-2 grid size-8 place-items-center rounded-md text-zinc-400 hover:bg-white/5 hover:text-white lg:hidden"
        onclick={() => (sidebarOpen = true)}
        type="button"
      >
        ☰
      </button>
      <a class="flex min-w-0 items-center gap-2" href="/dashboard">
        <span
          class="grid size-7 shrink-0 place-items-center rounded-md border border-amber-400/30 bg-amber-400/5"
        >
          <span class="size-2.5 rotate-45 rounded-[2px] bg-amber-400"></span>
        </span>
        <span class="truncate text-sm font-medium">Jet Black</span>
        <span class="text-xs text-zinc-600">⌄</span>
      </a>
    </div>

    <div class="flex min-w-0 flex-1 items-center justify-center px-3">
      <button
        class="flex h-8 w-full max-w-sm items-center gap-2 rounded-md border border-white/10 bg-[#191a1a] px-3 text-left text-xs text-zinc-500 transition hover:border-white/20 hover:text-zinc-400"
        type="button"
      >
        <span aria-hidden="true">⌕</span>
        <span class="truncate">Search commands…</span>
        <kbd class="ml-auto hidden font-mono text-[10px] text-zinc-600 sm:block">
          ⌘ K
        </kbd>
      </button>
    </div>

    <div class="flex items-center gap-2 px-3">
      <div
        class="grid size-7 place-items-center rounded-full bg-amber-400 text-[10px] font-bold text-black"
        title={data.user.email}
      >
        {initials}
      </div>
    </div>
  </header>

  {#if sidebarOpen}
    <button
      aria-label="Close navigation"
      class="fixed inset-0 z-30 bg-black/60 lg:hidden"
      onclick={() => (sidebarOpen = false)}
      type="button"
    ></button>
  {/if}

  <aside
    class="fixed bottom-0 left-0 top-12 z-40 flex w-[248px] flex-col border-r border-white/10 bg-[#111212] transition-transform lg:translate-x-0 {sidebarOpen
      ? 'translate-x-0'
      : '-translate-x-full'}"
  >
    <div class="flex items-center justify-between px-4 py-4">
      <h2 class="text-base font-medium">Projects</h2>
      <button
        aria-label="Close navigation"
        class="text-zinc-500 lg:hidden"
        onclick={() => (sidebarOpen = false)}
        type="button"
      >
        ✕
      </button>
    </div>

    <nav class="flex-1 space-y-5 overflow-y-auto px-3 pb-4 text-sm">
      <div class="space-y-1">
        <button
          class="mb-3 flex h-9 w-full items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] px-3 text-left text-zinc-300 transition hover:border-white/20 hover:bg-white/[0.06]"
          type="button"
        >
          <span aria-hidden="true">✧</span>
          New work item
        </button>
        <a
          class="flex h-8 items-center gap-2 rounded-md bg-white/10 px-3 text-zinc-100"
          href="/dashboard"
        >
          <span aria-hidden="true">⌂</span>
          Home
        </a>
        <a
          class="flex h-8 items-center gap-2 rounded-md px-3 text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200"
          href="/dashboard"
        >
          <span aria-hidden="true">◇</span>
          Stickies
        </a>
      </div>

      <div>
        <p class="mb-2 px-2 text-xs font-medium text-zinc-600">Workspace</p>
        <a
          class="flex h-8 items-center gap-2 rounded-md px-3 text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200"
          href="/dashboard"
        >
          <span aria-hidden="true">▣</span>
          Projects
        </a>
        <a
          class="flex h-8 items-center gap-2 rounded-md px-3 text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200"
          href="/dashboard"
        >
          <span aria-hidden="true">•••</span>
          More
        </a>
      </div>

      <div>
        <p class="mb-2 px-2 text-xs font-medium text-zinc-600">Projects</p>
        <div
          class="rounded-md border border-dashed border-white/10 px-3 py-4 text-xs leading-5 text-zinc-600"
        >
          Your projects will appear here.
        </div>
      </div>
    </nav>

    <div class="border-t border-white/10 p-3">
      <div class="mb-3 flex min-w-0 items-center gap-2 px-1">
        <div
          class="grid size-7 shrink-0 place-items-center rounded-full bg-amber-400 text-[10px] font-bold text-black"
        >
          {initials}
        </div>
        <div class="min-w-0">
          <p class="truncate text-xs font-medium text-zinc-300">
            {data.user.name ?? "Jet Black user"}
          </p>
          <p class="truncate text-[10px] text-zinc-600">{data.user.email}</p>
        </div>
      </div>
      <button
        class="flex h-8 w-full items-center rounded-md px-3 text-left text-xs text-zinc-500 transition hover:bg-white/5 hover:text-zinc-300 disabled:opacity-50"
        disabled={signingOut}
        onclick={signOut}
        type="button"
      >
        {signingOut ? "Signing out…" : "Sign out"}
      </button>
    </div>
  </aside>

  <main class="min-h-screen pt-12 lg:pl-[248px]">
    <div class="border-b border-white/10 px-5 py-4 sm:px-8">
      <div class="flex items-center gap-2 text-sm text-zinc-400">
        <span aria-hidden="true">⌂</span>
        Home
      </div>
    </div>

    <div class="mx-auto max-w-4xl px-5 py-12 sm:px-8 sm:py-16">
      <header class="text-center">
        <h1 class="text-xl font-semibold tracking-tight">
          Good to see you, {firstName}
        </h1>
        <p class="mt-1 text-sm text-zinc-500">
          Your real-time workspace is ready.
        </p>
      </header>

      <section class="mt-12">
        <div class="mb-4 flex items-center justify-between">
          <h2 class="text-sm font-medium text-zinc-400">Quicklinks</h2>
          <button
            class="text-xs text-amber-400 transition hover:text-amber-300"
            type="button"
          >
            + Add quick link
          </button>
        </div>
        <div
          class="grid min-h-48 place-items-center rounded-md border border-white/[0.06] bg-[#151616] p-8 text-center"
        >
          <div>
            <div
              class="mx-auto grid size-12 place-items-center rounded-xl border border-white/[0.06] bg-white/[0.02] text-xl text-zinc-700"
            >
              ↗
            </div>
            <p class="mt-4 max-w-sm text-sm text-zinc-500">
              Keep important references, resources, and documentation close to
              your work.
            </p>
          </div>
        </div>
      </section>

      <section class="mt-12">
        <div class="mb-3 flex items-center justify-between">
          <h2 class="text-sm font-medium text-zinc-400">Recent activity</h2>
          <span class="text-xs text-zinc-600">
            {data.connected ? "Live" : "Connecting…"}
          </span>
        </div>
        <div class="divide-y divide-white/[0.05]">
          {#each data.messages as item (item._id)}
            <article
              class="group flex items-center gap-3 px-2 py-3 text-sm transition hover:bg-white/[0.02]"
            >
              <span
                class="grid size-8 shrink-0 place-items-center rounded-md bg-white/[0.04] text-zinc-500"
              >
                ◇
              </span>
              <div class="min-w-0 flex-1">
                <p class="truncate text-zinc-300">{item.body}</p>
                <p class="mt-0.5 text-xs text-zinc-600">{item.source}</p>
              </div>
              <span
                class="size-2 rounded-full bg-emerald-400 opacity-60"
                title="Available"
              ></span>
            </article>
          {:else}
            <div
              class="rounded-md border border-dashed border-white/10 px-5 py-8 text-center text-sm text-zinc-600"
            >
              Activity from your workspace will appear here.
            </div>
          {/each}
        </div>
      </section>
    </div>
  </main>
</div>
