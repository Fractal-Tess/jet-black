<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { onMount } from "svelte";
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";

let { data } = $props();

let mode: "sign-in" | "sign-up" = $state("sign-in");
let name = $state("");
let email = $state("");
let password = $state("");
let error = $state("");
let hydrated = $state(false);
let submitting = $state(false);

onMount(() => {
  hydrated = true;
});

async function submit(event: SubmitEvent) {
  event.preventDefault();
  error = "";
  submitting = true;

  try {
    const result =
      mode === "sign-up"
        ? await authClient.signUp.email({
            email: email.trim(),
            name: name.trim(),
            password,
          })
        : await authClient.signIn.email({
            email: email.trim(),
            password,
          });

    if (result.error) {
      error = result.error.message ?? "We could not authenticate this account.";
      return;
    }

    await goto(data.returnTo);
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Authentication is temporarily unavailable.";
  } finally {
    submitting = false;
  }
}

function switchMode(nextMode: "sign-in" | "sign-up") {
  mode = nextMode;
  error = "";
  password = "";
}
</script>

<svelte:head>
  <title>{mode === "sign-in" ? "Sign in" : "Create account"} · Jet Black</title>
  <meta
    name="description"
    content="Sign in to your Jet Black workspace."
  />
</svelte:head>

<main
  class="relative flex min-h-screen flex-col overflow-hidden bg-background text-foreground"
  data-hydrated={hydrated}
>
  <div
    aria-hidden="true"
    class="pointer-events-none absolute inset-0 bg-radial from-primary/5 via-transparent to-transparent"
  ></div>

  <header class="relative flex h-14 shrink-0 items-center justify-between px-5 sm:px-8">
    <a
      class="group flex items-center gap-2.5 text-lg font-medium tracking-tight"
      href="/"
    >
      <span
        class="grid size-7 place-items-center rounded-md border border-primary/30 bg-primary/5 transition-colors group-hover:border-primary/60"
      >
        <img
          alt=""
          class="size-5 object-contain"
          height="20"
          src="/brand/logo-mark-light.png"
          width="20"
        />
      </span>
      Jet Black
    </a>

    <p class="hidden text-sm text-muted-foreground sm:block">
      {mode === "sign-in" ? "New to Jet Black?" : "Already have an account?"}
      <Button
        class="ml-1 h-auto p-0 text-sm"
        onclick={() =>
          switchMode(mode === "sign-in" ? "sign-up" : "sign-in")}
        variant="link"
      >
        {mode === "sign-in" ? "Create an account" : "Sign in"}
      </Button>
    </p>
  </header>

  <section class="relative flex flex-1 items-center justify-center px-5 py-12">
    <div class="w-full max-w-sm">
      <div class="mb-7">
        <h1 class="text-xl font-semibold tracking-tight">
          {mode === "sign-in"
            ? "Work without the refresh button."
            : "Build your real-time workspace."}
        </h1>
        <p class="mt-1 text-xl font-semibold tracking-tight text-muted-foreground">
          {mode === "sign-in"
            ? "Welcome back to Jet Black."
            : "Create your Jet Black account."}
        </p>
      </div>

      <Card class="p-6">
      <form class="space-y-4" onsubmit={submit}>
        {#if mode === "sign-up"}
          <label class="block" for="name">
            <span class="mb-1.5 block text-sm text-foreground">Name</span>
            <Input
              autocomplete="name"
              class="h-10"
              id="name"
              minlength={2}
              bind:value={name}
              placeholder="Your name"
              required
            />
          </label>
        {/if}

        <label class="block" for="email">
          <span class="mb-1.5 block text-sm text-foreground">Email</span>
          <Input
            autocomplete="email"
            class="h-10"
            id="email"
            bind:value={email}
            placeholder="name@company.com"
            required
            type="email"
          />
        </label>

        <label class="block" for="password">
          <span class="mb-1.5 block text-sm text-foreground">Password</span>
          <Input
            autocomplete={mode === "sign-in"
              ? "current-password"
              : "new-password"}
            class="h-10"
            id="password"
            minlength={8}
            bind:value={password}
            placeholder="At least 8 characters"
            required
            type="password"
          />
        </label>

        <div aria-live="polite" class="min-h-5">
          {#if error}
            <p class="text-sm text-destructive" role="alert">{error}</p>
          {/if}
        </div>

        <Button
          class="h-10 w-full"
          disabled={submitting}
          type="submit"
        >
          {#if submitting}
            <span
              aria-hidden="true"
              class="size-4 animate-spin rounded-full border-2 border-current border-r-transparent"
            ></span>
          {/if}
          {submitting
            ? "Please wait…"
            : mode === "sign-in"
              ? "Sign in"
              : "Create account"}
        </Button>
      </form>
      </Card>

      <p class="mt-5 text-center text-sm text-muted-foreground sm:hidden">
        {mode === "sign-in" ? "New to Jet Black?" : "Already registered?"}
        <Button
          class="ml-1 h-auto p-0 text-sm"
          onclick={() =>
            switchMode(mode === "sign-in" ? "sign-up" : "sign-in")}
          variant="link"
        >
          {mode === "sign-in" ? "Create an account" : "Sign in"}
        </Button>
      </p>

      <p class="mt-7 text-center text-xs leading-5 text-muted-foreground">
        By continuing, you agree to our
        <a class="text-foreground underline hover:text-primary" href="/terms">
          Terms of Service
        </a>
        and
        <a class="text-foreground underline hover:text-primary" href="/privacy">
          Privacy Policy
        </a>.
      </p>
    </div>
  </section>

  <footer
    class="relative flex shrink-0 items-center justify-center px-5 py-6 text-xs text-muted-foreground"
  >
    <span class="mr-2 inline-block size-1.5 rounded-full bg-success"></span>
    Real-time workspaces powered by Convex
  </footer>
</main>
