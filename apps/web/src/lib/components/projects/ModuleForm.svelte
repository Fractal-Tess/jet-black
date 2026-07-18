<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import {
  Content as SelectContent,
  Item as SelectItem,
  Root as SelectRoot,
  Trigger as SelectTrigger,
} from "@workspace/ui/components/select";
import { Textarea } from "@workspace/ui/components/textarea";
import type { ProjectModuleRecord } from "$lib/components/issues/types";

let {
  creating,
  onCreate,
}: {
  creating: boolean;
  onCreate: (input: {
    description?: string;
    name: string;
    status?: ProjectModuleRecord["status"];
    targetDate?: string;
  }) => Promise<void>;
} = $props();

let description = $state("");
let name = $state("");
let status = $state<ProjectModuleRecord["status"]>("planned");
let targetDate = $state("");

async function createModule() {
  const nextName = name.trim();

  if (!nextName) {
    return;
  }

  await onCreate({
    description: description.trim() || undefined,
    name: nextName,
    status,
    targetDate: targetDate || undefined,
  });
  description = "";
  name = "";
  status = "planned";
  targetDate = "";
}
</script>

<Card>
  <div class="border-b border-border px-4 py-3">
    <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
      Modules
    </p>
    <h2 class="mt-1 text-lg font-semibold text-foreground">
      New module
    </h2>
  </div>

  <form
    class="grid gap-3 p-4"
    onsubmit={(event) => {
      event.preventDefault();
      createModule();
    }}
  >
    <div>
      <Label class="mb-1">Module name</Label>
      <Input
        bind:value={name}
        placeholder="Billing, Inbox, API"
      />
    </div>

    <div class="grid gap-3 sm:grid-cols-2">
      <div>
        <Label class="mb-1">Status</Label>
        <SelectRoot type="single" value={status} onValueChange={(v) => { if (v) status = v as ProjectModuleRecord["status"]; }}>
          <SelectTrigger class="w-full">
            {status === "in_progress" ? "In progress" : status.charAt(0).toUpperCase() + status.slice(1)}
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="backlog">Backlog</SelectItem>
            <SelectItem value="planned">Planned</SelectItem>
            <SelectItem value="in_progress">In progress</SelectItem>
            <SelectItem value="completed">Completed</SelectItem>
          </SelectContent>
        </SelectRoot>
      </div>
      <div>
        <Label class="mb-1">Target date</Label>
        <Input
          bind:value={targetDate}
          type="date"
          class="h-9"
        />
      </div>
    </div>

    <div>
      <Label class="mb-1">Module description</Label>
      <Textarea
        bind:value={description}
        class="min-h-20"
        placeholder="What work belongs in this module?"
      />
    </div>

    <Button
      disabled={creating || !name.trim()}
      type="submit"
    >
      {creating ? "Creating\u2026" : "Create module"}
    </Button>
  </form>
</Card>
