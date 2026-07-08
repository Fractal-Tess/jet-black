<script lang="ts">
import { Button } from "../lib/components/button";
import { Card } from "../lib/components/card";
import "../app.css";
</script>

<main class="grid min-h-screen place-items-center p-8">
  <Card class="max-w-lg p-8">
    <p class="font-mono text-xs uppercase tracking-[0.25em] text-primary">Shared UI</p>
    <h1 class="mt-3 text-4xl font-semibold">Svelte components</h1>
    <p class="mt-3 text-muted-foreground">The workspace UI package is ready for shadcn-svelte components.</p>
    <Button class="mt-6">Button</Button>
  </Card>
</main>
