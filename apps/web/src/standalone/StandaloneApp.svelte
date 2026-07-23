<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { onMount } from "svelte";
import { createBrowserCommandSession } from "../lib/execution-client/browser-command-session";
import type { StandaloneRunSetup } from "../lib/execution-client/standalone-setup";
import { startStandaloneRun } from "../lib/execution-client/standalone-setup";
import type { ExecutionClient } from "../lib/execution-client/types";

type ConnectionStatus = "connecting" | "ready" | "failed";

const connectionLabels: Record<ConnectionStatus, string> = {
  connecting: "Connecting",
  ready: "Connected",
  failed: "Connection failed",
};

let client = $state<ExecutionClient | null>(null);
let connectionStatus = $state<ConnectionStatus>("connecting");
let repositoryPath = $state("");
let setup = $state<StandaloneRunSetup | null>(null);
let setupAttempted = $state(false);
let busy = $state(false);
let errorMessage = $state<string | null>(null);

const errorText = (error: unknown): string =>
  error instanceof Error ? error.message : "Local execution setup failed.";

const connect = async (): Promise<void> => {
  try {
    client = await createBrowserCommandSession();
    connectionStatus = "ready";
  } catch (error) {
    connectionStatus = "failed";
    errorMessage = errorText(error);
  }
};

const startRun = async (event: SubmitEvent): Promise<void> => {
  event.preventDefault();
  if (client === null || busy) {
    return;
  }

  setupAttempted = true;
  busy = true;
  errorMessage = null;
  try {
    setup = await startStandaloneRun(client, repositoryPath);
  } catch (error) {
    errorMessage = errorText(error);
  } finally {
    busy = false;
  }
};

onMount(connect);
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
      <Badge variant={connectionStatus === "failed" ? "destructive" : "outline"}>
        {connectionLabels[connectionStatus]}
      </Badge>
    </header>

    <section class="grid flex-1 place-items-center py-16" aria-labelledby="launch-heading">
      <Card class="w-full max-w-2xl p-8">
        <p class="text-sm font-medium text-muted-foreground">Standalone workspace</p>
        <h2 id="launch-heading" class="mt-2 text-xl font-semibold">
          {setup === null ? "Start a local execution run" : "Local run started"}
        </h2>
        <p class="mt-3 max-w-xl text-sm leading-6 text-muted-foreground">
          Register an approved Git repository path. Jet Black creates an isolated changeset
          and starts the run using the exact repository revision returned by the local service.
        </p>

        {#if setup === null}
          <form class="mt-8 space-y-4" onsubmit={startRun}>
            <div class="space-y-2">
              <Label for="repository-path">Repository path</Label>
              <Input
                id="repository-path"
                name="repositoryPath"
                bind:value={repositoryPath}
                placeholder="/absolute/path/to/repository"
                autocomplete="off"
                spellcheck="false"
                disabled={connectionStatus !== "ready" || setupAttempted}
              />
            </div>

            {#if errorMessage !== null}
              <div id="setup-error" class="space-y-1 text-sm" role="alert">
                <p class="text-destructive">{errorMessage}</p>
                {#if setupAttempted}
                  <p class="text-muted-foreground">
                    For safety, this page will not repeat a partially completed setup.
                  </p>
                {/if}
              </div>
            {/if}

            <Button
              type="submit"
              disabled={connectionStatus !== "ready" || setupAttempted || repositoryPath.trim().length === 0}
            >
              {busy ? "Starting run…" : setupAttempted ? "Setup stopped" : "Start local run"}
            </Button>
          </form>
        {:else}
          <dl class="mt-8 grid gap-4 rounded-lg border border-border bg-muted/30 p-4 text-sm">
            <div>
              <dt class="text-muted-foreground">Repository</dt>
              <dd class="mt-1 break-all font-mono">{setup.repository.canonical_path}</dd>
            </div>
            <div>
              <dt class="text-muted-foreground">Changeset</dt>
              <dd class="mt-1 break-all font-mono">{setup.changeset.id}</dd>
            </div>
            <div>
              <dt class="text-muted-foreground">Run</dt>
              <dd class="mt-1 break-all font-mono">{setup.run.run_id}</dd>
            </div>
          </dl>
        {/if}
      </Card>
    </section>
  </div>
</main>
