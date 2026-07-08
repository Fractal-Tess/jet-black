<script lang="ts">
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";

let { data } = $props();
async function signOut() {
  await authClient.signOut();
  await goto("/login");
}
</script>

<svelte:head><title>Dashboard · Jet Black</title></svelte:head>

<main class="min-h-screen">
  <nav class="flex h-16 items-center justify-between border-b px-6 md:px-10">
    <a class="font-mono text-sm uppercase tracking-[0.16em]" href="/">Jet Black</a>
    <button class="text-sm text-muted-foreground hover:text-foreground" onclick={signOut}>Sign out</button>
  </nav>
  <div class="mx-auto max-w-7xl space-y-8 p-6 md:p-10">
    <header>
      <p class="font-mono text-xs uppercase tracking-[0.3em] text-primary">Dashboard</p>
      <h1 class="mt-3 text-4xl font-semibold">Welcome back{data.user.name ? `, ${data.user.name}` : ""}.</h1>
      <p class="mt-2 text-muted-foreground">Your Better Auth session with Convex-backed workspace data.</p>
    </header>
    <section class="grid gap-4 md:grid-cols-3">
      <div class="rounded-2xl border bg-card p-5"><p class="text-xs uppercase tracking-wider text-muted-foreground">Current user</p><p class="mt-3 truncate font-medium">{data.user.email}</p></div>
      <div class="rounded-2xl border bg-card p-5"><p class="text-xs uppercase tracking-wider text-muted-foreground">Messages</p><p class="mt-3 text-3xl font-semibold">{data.messages.length}</p></div>
      <div class="rounded-2xl border bg-card p-5"><p class="text-xs uppercase tracking-wider text-muted-foreground">Recent scrapes</p><p class="mt-3 text-3xl font-semibold">{data.scrapes.length}</p></div>
    </section>
    <section class="grid gap-5 lg:grid-cols-2">
      <div class="overflow-hidden rounded-2xl border bg-card"><h2 class="border-b p-5 text-lg font-semibold">Latest messages</h2>{#each data.messages as item (item._id)}<article class="border-b p-5 last:border-0"><p class="text-sm font-medium">{item.source}</p><p class="mt-2 text-sm text-muted-foreground">{item.body}</p></article>{/each}</div>
      <div class="overflow-hidden rounded-2xl border bg-card"><h2 class="border-b p-5 text-lg font-semibold">Recent scrape runs</h2>{#each data.scrapes as item (item._id)}<article class="border-b p-5 last:border-0"><p class="text-sm font-medium">{item.mode}</p><p class="mt-2 text-sm text-muted-foreground">{item.summary}</p></article>{/each}</div>
    </section>
  </div>
</main>
