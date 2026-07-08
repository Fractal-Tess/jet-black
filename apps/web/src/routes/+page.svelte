<script lang="ts">
let { data } = $props();
</script>

<svelte:head><title>Jet Black</title></svelte:head>

<main class="min-h-screen overflow-hidden">
  <nav class="flex h-16 items-center justify-between border-b bg-background/80 px-6 backdrop-blur-xl md:px-12">
    <a class="font-mono text-sm font-semibold uppercase tracking-[0.16em]" href="/">Jet Black</a>
    <div class="flex items-center gap-5 text-sm">
      <a class="text-muted-foreground transition hover:text-foreground" href="/dashboard">Dashboard</a>
      <a class="rounded-full bg-primary px-4 py-2 font-medium text-primary-foreground" href="/login">Login</a>
    </div>
  </nav>

  <section class="relative mx-auto grid max-w-7xl gap-10 px-6 py-20 md:px-12 lg:grid-cols-[1.15fr_0.85fr]">
    <div class="pointer-events-none absolute -left-40 top-10 h-96 w-96 rounded-full bg-primary/10 blur-[120px]"></div>
    <div class="relative self-center">
      <p class="font-mono text-xs uppercase tracking-[0.3em] text-primary">SvelteKit · Convex · Bun</p>
      <h1 class="mt-5 max-w-3xl text-5xl font-semibold leading-[0.98] tracking-tight md:text-7xl">A minimal starter for apps, auth, workers & scraping.</h1>
      <p class="mt-7 max-w-2xl text-lg leading-relaxed text-muted-foreground">Shared Svelte UI, server-rendered routes, Better Auth, and live Convex data in one focused workspace.</p>
      <div class="mt-9 flex gap-3">
        <a class="rounded-full bg-primary px-6 py-3 font-medium text-primary-foreground transition hover:-translate-y-0.5" href="/login">Get started</a>
        <a class="rounded-full border px-6 py-3 font-medium transition hover:bg-accent" href="/dashboard">Open dashboard</a>
      </div>
    </div>

    <aside class="rounded-3xl border bg-card/80 p-6 shadow-2xl shadow-black/25 backdrop-blur">
      <div class="flex items-center justify-between">
        <p class="font-mono text-xs uppercase tracking-[0.22em] text-muted-foreground">Live preview</p>
        <span class="rounded-full px-2.5 py-1 text-xs {data.connected ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'}">{data.connected ? "Connected" : "Offline"}</span>
      </div>
      <div class="mt-6 space-y-3">
        {#each data.messages as message (message._id)}
          <article class="rounded-2xl border bg-background/50 p-4">
            <p class="font-mono text-[0.65rem] uppercase text-primary">{message.source}</p>
            <p class="mt-2 text-sm leading-relaxed text-muted-foreground">{message.body}</p>
          </article>
        {:else}
          <div class="rounded-2xl border border-dashed p-6 text-sm text-muted-foreground">Start Convex to populate the live data preview.</div>
        {/each}
      </div>
    </aside>
  </section>
</main>
