<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Card } from "@workspace/ui/components/card";

const connectionStatus = "Connecting";
</script>

<svelte:head>
  <meta
    name="description"
    content="Local-first Jet Black execution and review workspace"
  />
</svelte:head>

<main class="min-h-screen bg-background text-foreground">
  <div class="mx-auto flex min-h-screen max-w-6xl flex-col px-6 py-8">
    <header class="flex items-center justify-between border-b border-border pb-6">
      <div>
        <p class="text-xs font-medium tracking-[0.2em] text-muted-foreground uppercase">
          Jet Black
        </p>
        <h1 class="mt-2 text-2xl font-semibold">Local execution</h1>
      </div>
      <Badge variant="outline">{connectionStatus}</Badge>
    </header>

    <section class="grid flex-1 place-items-center py-16" aria-labelledby="launch-heading">
      <Card class="w-full max-w-2xl p-8">
        <p class="text-sm font-medium text-muted-foreground">Standalone workspace</p>
        <h2 id="launch-heading" class="mt-2 text-xl font-semibold">
          Preparing the local execution session
        </h2>
        <p class="mt-3 max-w-xl text-sm leading-6 text-muted-foreground">
          This page connects only to the loopback Jet Black service. Repository setup,
          run controls, review, and finalization will appear here when the session is ready.
        </p>
      </Card>
    </section>
  </div>
</main>
