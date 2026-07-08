<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { greet } from "./lib/bindings";

let name = $state("Fractal");
let greeting = $state("Hello from Tauri.");

async function submit(event: SubmitEvent) {
  event.preventDefault();
  greeting = await greet(name);
}
</script>

<main class="grid min-h-screen place-items-center bg-background p-8">
  <Card class="w-full max-w-xl p-8 shadow-2xl shadow-black/25">
    <p class="font-mono text-xs uppercase tracking-[0.25em] text-primary">Svelte + Tauri</p>
    <h1 class="mt-3 text-3xl font-semibold">Desktop app</h1>
    <p class="mt-2 text-muted-foreground">Uses shared Svelte components from <code>@workspace/ui</code>.</p>
    <form class="mt-7 flex flex-col gap-3 sm:flex-row" onsubmit={submit}>
      <label class="sr-only" for="name">Name</label>
      <Input id="name" bind:value={name} placeholder="Enter your name" />
      <Button type="submit">Greet</Button>
    </form>
    <p class="mt-4 text-muted-foreground">{greeting}</p>
  </Card>
</main>
