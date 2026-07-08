<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Input } from "@workspace/ui/components/input";
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";

let mode: "sign-in" | "sign-up" = $state("sign-in");
let name = $state("");
let email = $state("");
let password = $state("");
let error = $state("");
let submitting = $state(false);

async function submit(event: SubmitEvent) {
  event.preventDefault();
  error = "";
  submitting = true;
  try {
    const result =
      mode === "sign-up"
        ? await authClient.signUp.email({ email, name, password })
        : await authClient.signIn.email({ email, password });
    if (result.error) {
      error = result.error.message ?? "Authentication failed.";
      return;
    }
    await goto("/dashboard");
  } finally {
    submitting = false;
  }
}
</script>

<svelte:head><title>Login · Jet Black</title></svelte:head>

<main class="grid min-h-screen lg:grid-cols-[30rem_1fr]">
  <section class="flex items-center justify-center p-6 md:p-10">
    <form class="w-full max-w-md rounded-3xl border bg-card p-8" onsubmit={submit}>
      <p class="text-center font-mono text-xs uppercase tracking-[0.25em] text-primary">{mode === "sign-in" ? "Welcome back" : "Get started"}</p>
      <h1 class="mt-3 text-center text-3xl font-semibold">{mode === "sign-in" ? "Sign in to your account" : "Create your account"}</h1>
      <p class="mt-2 text-center text-sm text-muted-foreground">Better Auth against your Convex deployment.</p>
      <div class="mt-7 grid grid-cols-2 rounded-full border bg-muted/50 p-1">
        <Button type="button" variant={mode === "sign-in" ? "default" : "ghost"} onclick={() => mode = "sign-in"}>Sign in</Button>
        <Button type="button" variant={mode === "sign-up" ? "default" : "ghost"} onclick={() => mode = "sign-up"}>Create account</Button>
      </div>
      <div class="mt-6 space-y-4">
        {#if mode === "sign-up"}
          <label class="grid gap-2 text-sm" for="name">Name<Input id="name" bind:value={name} required placeholder="Ada Lovelace" /></label>
        {/if}
        <label class="grid gap-2 text-sm" for="email">Email<Input id="email" bind:value={email} required type="email" placeholder="ada@example.com" /></label>
        <label class="grid gap-2 text-sm" for="password">Password<Input id="password" bind:value={password} minlength={8} required type="password" /></label>
        {#if error}<p class="rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-sm text-red-300">{error}</p>{/if}
        <Button class="w-full" disabled={submitting} type="submit">{submitting ? "Working…" : mode === "sign-in" ? "Sign in" : "Create account"}</Button>
      </div>
      <p class="mt-6 text-center text-sm text-muted-foreground">Need context first? <a class="underline" href="/">Return home</a>.</p>
    </form>
  </section>
  <section class="relative hidden overflow-hidden border-l bg-card lg:block">
    <div class="absolute inset-0 bg-[radial-gradient(circle_at_35%_35%,color-mix(in_oklch,var(--primary)_20%,transparent),transparent_45%)]"></div>
    <div class="absolute bottom-12 left-12 max-w-xl"><p class="font-mono text-xs uppercase tracking-[0.3em] text-primary">Svelte, end to end</p><h2 class="mt-4 text-5xl font-semibold">One reactive stack.<br />No virtual DOM.</h2></div>
  </section>
</main>
