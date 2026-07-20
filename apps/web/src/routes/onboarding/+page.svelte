<script lang="ts">
import { api } from "@workspace/convex/api";
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
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
  class="relative flex min-h-screen flex-col overflow-hidden bg-background text-foreground"
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

    <p class="text-sm text-muted-foreground">
      {step === "profile" ? "Step 1 of 2" : "Step 2 of 2"}
    </p>
  </header>

  <section class="relative flex flex-1 items-center justify-center px-5 py-12">
    <div class="w-full max-w-md">
      {#if step === "profile"}
        <div class="mb-7">
          <h1 class="text-xl font-semibold tracking-tight">
            Welcome to Jet Black
          </h1>
          <p class="mt-1 text-sm text-muted-foreground">
            Let's get your profile set up first.
          </p>
        </div>

        <Card class="p-6">
        <form class="space-y-4" onsubmit={handleProfileSubmit}>
          <label class="block" for="name">
            <span class="mb-1.5 block text-sm text-foreground">Your name</span>
            <Input
              autocomplete="name"
              class="h-10"
              id="name"
              minlength={2}
              bind:value={name}
              placeholder="Your full name"
              required
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
            {submitting ? "Saving…" : "Continue"}
          </Button>
        </form>
        </Card>
      {:else}
        <div class="mb-7">
          <h1 class="text-xl font-semibold tracking-tight">
            Create your workspace
          </h1>
          <p class="mt-1 text-sm text-muted-foreground">
            A workspace is where your team collaborates on projects.
          </p>
        </div>

        <Card class="p-6">
        <form class="space-y-4" onsubmit={handleWorkspaceSubmit}>
          <label class="block" for="workspace-name">
            <span class="mb-1.5 block text-sm text-foreground">Workspace name</span>
            <Input
              autocomplete="organization"
              class="h-10"
              id="workspace-name"
              minlength={2}
              maxlength={80}
              value={workspaceName}
              oninput={(e) => handleWorkspaceNameChange(e.currentTarget.value)}
              placeholder="e.g. Acme Corp"
              required
            />
          </label>

          <label class="block" for="workspace-slug">
            <span class="mb-1.5 block text-sm text-foreground">Workspace URL</span>
            <div class="flex h-10 items-stretch rounded-lg border border-input bg-background transition-colors focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50">
              <span class="flex shrink-0 items-center border-r border-border bg-muted px-3 text-sm text-muted-foreground">
                jet-black.app/
              </span>
              <input
                class="min-w-0 flex-1 bg-transparent px-3 text-sm outline-none placeholder:text-muted-foreground"
                id="workspace-slug"
                minlength="2"
                maxlength="48"
                bind:value={workspaceSlug}
                placeholder={deriveSlug(workspaceName) || "your-workspace"}
                required
              />
            </div>
            <p class="mt-1 text-xs text-muted-foreground">
              This will be your workspace's unique URL. You can change it later.
            </p>
          </label>

          <div aria-live="polite" class="min-h-5">
            {#if error}
              <p class="text-sm text-destructive" role="alert">{error}</p>
            {/if}
          </div>

          <Button
            class="h-10 w-full"
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
          </Button>
        </form>
        </Card>
      {/if}
    </div>
  </section>

  <footer
    class="relative flex shrink-0 items-center justify-center px-5 py-6 text-xs text-muted-foreground"
  >
    <span class="mr-2 inline-block size-1.5 rounded-full bg-success"></span>
    Real-time workspaces powered by Convex
  </footer>
</main>
