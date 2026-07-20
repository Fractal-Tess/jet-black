<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { Textarea } from "@workspace/ui/components/textarea";
import type { IntakeIssue } from "$lib/components/issues/types";

let {
  creating,
  intakeIssues,
  onAccept,
  onCreate,
  onDecline,
}: {
  creating: boolean;
  intakeIssues: IntakeIssue[];
  onAccept: (intakeIssue: IntakeIssue) => Promise<void>;
  onCreate: (input: {
    description?: string;
    source?: string;
    title: string;
  }) => Promise<void>;
  onDecline: (intakeIssue: IntakeIssue) => Promise<void>;
} = $props();

let description = $state("");
let source = $state("manual");
let title = $state("");

async function createIntakeIssue() {
  const nextTitle = title.trim();

  if (!nextTitle) {
    return;
  }

  await onCreate({
    description: description.trim() || undefined,
    source: source.trim() || "manual",
    title: nextTitle,
  });
  description = "";
  source = "manual";
  title = "";
}
</script>

<section class="space-y-5">
  <Card>
    <div class="border-b border-border px-4 py-3">
      <p class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        Intake
      </p>
      <h2 class="mt-1 text-lg font-semibold text-foreground">New intake item</h2>
    </div>

    <form
      class="grid gap-3 p-4"
      onsubmit={(event) => {
        event.preventDefault();
        createIntakeIssue();
      }}
    >
      <div>
        <Label class="mb-1" for="intake-title">Intake title</Label>
        <Input
          bind:value={title}
          id="intake-title"
          placeholder="Describe the incoming request"
        />
      </div>
      <div>
        <Label class="mb-1" for="intake-source">Intake source</Label>
        <Input
          bind:value={source}
          class="h-9"
          id="intake-source"
          placeholder="manual, support, sales"
        />
      </div>
      <div>
        <Label class="mb-1" for="intake-description">Intake description</Label>
        <Textarea
          bind:value={description}
          class="min-h-20"
          id="intake-description"
          placeholder="Add context before triage"
        />
      </div>
      <Button
        disabled={creating || !title.trim()}
        type="submit"
      >
        {creating ? "Adding\u2026" : "Add intake item"}
      </Button>
    </form>
  </Card>

  <Card>
    <div
      class="flex items-center justify-between border-b border-border px-4 py-3"
    >
      <div>
      <p class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Triage
        </p>
        <h2 class="mt-1 text-lg font-semibold text-foreground">Inbox</h2>
      </div>
      <Badge variant="outline">
        {intakeIssues.length} total
      </Badge>
    </div>

    <div class="divide-y divide-border">
      {#each intakeIssues as intakeIssue (intakeIssue._id)}
        <article class="p-4">
          <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <Badge variant="secondary">
                  {intakeIssue.status}
                </Badge>
          <span class="font-mono text-meta text-muted-foreground">
                  {intakeIssue.source}
                </span>
              </div>
              <h3 class="mt-2 text-sm font-medium text-foreground">
                {intakeIssue.title}
              </h3>
              {#if intakeIssue.description}
                <p class="mt-1 text-sm text-muted-foreground">
                  {intakeIssue.description}
                </p>
              {/if}
            </div>

            {#if intakeIssue.status === "pending"}
              <div class="flex shrink-0 gap-2">
                <Button
                  variant="outline"
                  class="h-8 px-3 text-xs"
                  onclick={() => onDecline(intakeIssue)}
                >
                  Decline
                </Button>
                <Button
                  class="h-8 px-3 text-xs"
                  onclick={() => onAccept(intakeIssue)}
                >
                  Accept
                </Button>
              </div>
            {:else if intakeIssue.acceptedIssueId}
              <span class="text-xs text-muted-foreground">Accepted as issue</span>
            {/if}
          </div>
        </article>
      {:else}
        <p class="p-8 text-center text-sm text-muted-foreground">
          No intake items yet.
        </p>
      {/each}
    </div>
  </Card>
</section>
