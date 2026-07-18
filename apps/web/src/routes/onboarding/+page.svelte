<script lang="ts">
import { api } from "@workspace/convex/api";
import { useMutation } from "convex-svelte";
import { untrack } from "svelte";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { authClient } from "$lib/auth-client";
import { slugify } from "./slugs";

let { data } = $props();

// Skip to workspace step if ?step=workspace (e.g. from "Create workspace" in switcher)
const initialStep =
  page.url.searchParams.get("step") === "workspace" ? "workspace" : "profile";
let step = $state<"profile" | "workspace">(initialStep);
let name = $state(untrack(() => data.user.name ?? ""));
let workspaceName = $state("");
let workspaceSlug = $state("");
let error = $state("");
let submitting = $state(false);

const createWorkspace = useMutation(api.mutations.workspaces.createWorkspace);

function deriveSlug(value: string) {
  return slugify(value);
}

function handleWorkspaceNameChange(value: string) {
  workspaceName = value;
  workspaceSlug = deriveSlug(value);
}

async function handleProfileSubmit(e: SubmitEvent) {
  e.preventDefault();
  error = "";

  if (!name.trim()) {
    error = "Your name is required.";
    return;
  }

  submitting = true;

  try {
    await authClient.updateUser({ name: name.trim() });
    step = "workspace";
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Could not save your name. Please try again.";
  } finally {
    submitting = false;
  }
}

async function handleWorkspaceSubmit(e: SubmitEvent) {
  e.preventDefault();
  error = "";

  if (!workspaceName.trim()) {
    error = "Workspace name is required.";
    return;
  }

  if (!workspaceSlug) {
    error = "Workspace URL is required.";
    return;
  }

  submitting = true;

  try {
    const workspace = await createWorkspace({
      name: workspaceName.trim(),
      slug: workspaceSlug,
    });

    if (workspace) {
      await goto(`/workspace/${workspace.slug}`);
    } else {
      error = "Could not create workspace. Please try again.";
    }
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Could not create workspace. Please try again.";
  } finally {
    submitting = false;
  }
}
</script>

<svelte:head>
  <title>Set up your workspace · Jet Black</title>
  <meta
    name="description"
    content="Set up your Jet Black workspace."
  />
</svelte:head>

<main
  class="relative flex min-h-screen flex-col overflow-hidden bg-[#0d0e0e] text-[#f0f0ef]"
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

    <p class="text-sm text-zinc-500">
      {step === "profile" ? "Step 1 of 2" : "Step 2 of 2"}
    </p>
  </header>

  <section class="relative flex flex-1 items-center justify-center px-5 py-12">
    <div class="w-full max-w-[420px]">
      {#if step === "profile"}
        <div class="mb-7">
          <h1 class="text-xl font-semibold tracking-[-0.02em]">
            Welcome to Jet Black
          </h1>
          <p class="mt-1 text-sm text-zinc-500">
            Let's get your profile set up first.
          </p>
        </div>

        <form class="space-y-4" onsubmit={handleProfileSubmit}>
          <label class="block" for="name">
            <span class="mb-1.5 block text-sm text-zinc-300">Your name</span>
            <input
              autocomplete="name"
              class="h-11 w-full rounded-md border border-zinc-700 bg-[#181919] px-3 text-sm outline-none transition placeholder:text-zinc-600 hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
              id="name"
              minlength="2"
              bind:value={name}
              placeholder="Your full name"
              required
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
            {submitting ? "Saving…" : "Continue"}
          </button>
        </form>
      {:else}
        <div class="mb-7">
          <h1 class="text-xl font-semibold tracking-[-0.02em]">
            Create your workspace
          </h1>
          <p class="mt-1 text-sm text-zinc-500">
            A workspace is where your team collaborates on projects.
          </p>
        </div>

        <form class="space-y-4" onsubmit={handleWorkspaceSubmit}>
          <label class="block" for="workspace-name">
            <span class="mb-1.5 block text-sm text-zinc-300">Workspace name</span>
            <input
              autocomplete="organization"
              class="h-11 w-full rounded-md border border-zinc-700 bg-[#181919] px-3 text-sm outline-none transition placeholder:text-zinc-600 hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
              id="workspace-name"
              minlength="2"
              maxlength="80"
              value={workspaceName}
              oninput={(e) => handleWorkspaceNameChange(e.currentTarget.value)}
              placeholder="e.g. Acme Corp"
              required
            />
          </label>

          <label class="block" for="workspace-slug">
            <span class="mb-1.5 block text-sm text-zinc-300">Workspace URL</span>
            <div class="flex h-11 items-stretch rounded-md border border-zinc-700 bg-[#181919] focus-within:border-amber-400/70 focus-within:ring-2 focus-within:ring-amber-400/10 hover:border-zinc-600">
              <span class="flex shrink-0 items-center border-r border-zinc-700 bg-zinc-800/50 px-3 text-sm text-zinc-500">
                jet-black.app/
              </span>
              <input
                class="min-w-0 flex-1 bg-transparent px-3 text-sm outline-none placeholder:text-zinc-600"
                id="workspace-slug"
                minlength="2"
                maxlength="48"
                bind:value={workspaceSlug}
                placeholder={deriveSlug(workspaceName) || "your-workspace"}
                required
              />
            </div>
            <p class="mt-1 text-xs text-zinc-600">
              This will be your workspace's unique URL. You can change it later.
            </p>
          </label>

          <div aria-live="polite" class="min-h-5">
            {#if error}
              <p class="text-sm text-red-400" role="alert">{error}</p>
            {/if}
          </div>

          <button
            class="flex h-11 w-full items-center justify-center gap-2 rounded-md bg-amber-400 text-sm font-semibold text-black transition hover:bg-amber-300 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-amber-400 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-500"
            disabled={submitting || !workspaceName.trim() || !workspaceSlug}
            type="submit"
          >
            {#if submitting}
              <span
                aria-hidden="true"
                class="size-4 animate-spin rounded-full border-2 border-current border-r-transparent"
              ></span>
            {/if}
            {submitting ? "Creating workspace…" : "Create workspace"}
          </button>
        </form>
      {/if}
    </div>
  </section>

  <footer
    class="relative flex shrink-0 items-center justify-center px-5 py-6 text-xs text-zinc-600"
  >
    <span class="mr-2 inline-block size-1.5 rounded-full bg-emerald-400"></span>
    Real-time workspaces powered by Convex
  </footer>
</main>
