<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { Textarea } from "@workspace/ui/components/textarea";

let {
  creating,
  onCreate,
}: {
  creating: boolean;
  onCreate: (input: {
    description?: string;
    endDate?: string;
    name: string;
    startDate?: string;
  }) => Promise<void>;
} = $props();

let description = $state("");
let endDate = $state("");
let name = $state("");
let startDate = $state("");

async function createSprint() {
  const nextName = name.trim();

  if (!nextName) {
    return;
  }

  await onCreate({
    description: description.trim() || undefined,
    endDate: endDate || undefined,
    name: nextName,
    startDate: startDate || undefined,
  });
  description = "";
  endDate = "";
  name = "";
  startDate = "";
}
</script>

<Card>
  <div class="border-b border-border px-4 py-3">
    <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
      Sprints
    </p>
    <h2 class="mt-1 text-lg font-semibold text-foreground">New sprint</h2>
  </div>

  <form
    class="grid gap-3 p-4"
    onsubmit={(event) => {
      event.preventDefault();
      createSprint();
    }}
  >
    <div>
      <Label class="mb-1">Sprint name</Label>
      <Input
        bind:value={name}
        placeholder="Sprint 01"
      />
    </div>

    <div class="grid gap-3 sm:grid-cols-2">
      <div>
        <Label class="mb-1">Start date</Label>
        <Input
          bind:value={startDate}
          class="h-9"
          type="date"
        />
      </div>
      <div>
        <Label class="mb-1">End date</Label>
        <Input
          bind:value={endDate}
          class="h-9"
          type="date"
        />
      </div>
    </div>

    <div>
      <Label class="mb-1">Sprint description</Label>
      <Textarea
        bind:value={description}
        class="min-h-20"
        placeholder="Goal, scope, and handoff notes"
      />
    </div>

    <Button
      disabled={creating || !name.trim()}
      type="submit"
    >
      {creating ? "Creating\u2026" : "Create sprint"}
    </Button>
  </form>
</Card>
