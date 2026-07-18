<script lang="ts">
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
  class="relative flex min-h-screen flex-col overflow-hidden bg-[#0d0e0e] text-[#f0f0ef]"
  data-hydrated={hydrated}
>
  <div
    aria-hidden="true"
    class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_38%,rgba(255,184,0,0.045),transparent_28%)]"
  ></div>

  <header class="relative flex h-14 shrink-0 items-center justify-between px-5 sm:px-8">
    <a
      class="group flex items-center gap-2.5 text-lg font-medium tracking-tight"
      href="/"
    >
      <span
        class="grid size-7 place-items-center rounded-md border border-amber-400/30 bg-amber-400/5 transition group-hover:border-amber-400/60"
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

    <p class="hidden text-sm text-zinc-500 sm:block">
      {mode === "sign-in" ? "New to Jet Black?" : "Already have an account?"}
      <button
        class="ml-1 text-amber-400 transition hover:text-amber-300"
        onclick={() =>
          switchMode(mode === "sign-in" ? "sign-up" : "sign-in")}
        type="button"
      >
        {mode === "sign-in" ? "Create an account" : "Sign in"}
      </button>
    </p>
  </header>

  <section class="relative flex flex-1 items-center justify-center px-5 py-12">
    <div class="w-full max-w-[360px]">
      <div class="mb-7">
        <h1 class="text-xl font-semibold tracking-[-0.02em]">
          {mode === "sign-in"
            ? "Work without the refresh button."
            : "Build your real-time workspace."}
        </h1>
        <p class="mt-1 text-xl font-semibold tracking-[-0.02em] text-zinc-500">
          {mode === "sign-in"
            ? "Welcome back to Jet Black."
            : "Create your Jet Black account."}
        </p>
      </div>

      <form class="space-y-4" onsubmit={submit}>
        {#if mode === "sign-up"}
          <label class="block" for="name">
            <span class="mb-1.5 block text-sm text-zinc-300">Name</span>
            <input
              autocomplete="name"
              class="h-11 w-full rounded-md border border-zinc-700 bg-[#181919] px-3 text-sm outline-none transition placeholder:text-zinc-600 hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
              id="name"
              minlength="2"
              bind:value={name}
              placeholder="Your name"
              required
            />
          </label>
        {/if}

        <label class="block" for="email">
          <span class="mb-1.5 block text-sm text-zinc-300">Email</span>
          <input
            autocomplete="email"
            class="h-11 w-full rounded-md border border-zinc-700 bg-[#181919] px-3 text-sm outline-none transition placeholder:text-zinc-600 hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
            id="email"
            bind:value={email}
            placeholder="name@company.com"
            required
            type="email"
          />
        </label>

        <label class="block" for="password">
          <span class="mb-1.5 block text-sm text-zinc-300">Password</span>
          <input
            autocomplete={mode === "sign-in"
              ? "current-password"
              : "new-password"}
            class="h-11 w-full rounded-md border border-zinc-700 bg-[#181919] px-3 text-sm outline-none transition placeholder:text-zinc-600 hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
            id="password"
            minlength="8"
            bind:value={password}
            placeholder="At least 8 characters"
            required
            type="password"
          />
        </label>

        <div aria-live="polite" class="min-h-5">
          {#if error}
            <p class="text-sm text-red-400" role="alert">{error}</p>
          {/if}
        </div>

        <button
          class="flex h-11 w-full items-center justify-center gap-2 rounded-md bg-amber-400 text-sm font-semibold text-black transition hover:bg-amber-300 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-amber-400 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-500"
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
        </button>
      </form>

      <p class="mt-5 text-center text-sm text-zinc-500 sm:hidden">
        {mode === "sign-in" ? "New to Jet Black?" : "Already registered?"}
        <button
          class="ml-1 text-amber-400"
          onclick={() =>
            switchMode(mode === "sign-in" ? "sign-up" : "sign-in")}
          type="button"
        >
          {mode === "sign-in" ? "Create an account" : "Sign in"}
        </button>
      </p>

      <p class="mt-7 text-center text-xs leading-5 text-zinc-600">
        By continuing, you agree to our
        <a class="text-zinc-400 underline hover:text-zinc-300" href="/terms">
          Terms of Service
        </a>
        and
        <a class="text-zinc-400 underline hover:text-zinc-300" href="/privacy">
          Privacy Policy
        </a>.
      </p>
    </div>
  </section>

  <footer
    class="relative flex shrink-0 items-center justify-center px-5 py-6 text-xs text-zinc-600"
  >
    <span class="mr-2 inline-block size-1.5 rounded-full bg-emerald-400"></span>
    Real-time workspaces powered by Convex
  </footer>
</main>
