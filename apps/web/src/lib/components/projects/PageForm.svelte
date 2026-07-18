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
    content?: string;
    icon?: string;
    title: string;
  }) => Promise<void>;
} = $props();

let content = $state("");
let icon = $state("\u25a3");
let title = $state("");

async function createPage() {
  const nextTitle = title.trim();

  if (!nextTitle) {
    return;
  }

  await onCreate({
    content: content.trim(),
    icon: icon.trim() || undefined,
    title: nextTitle,
  });
  content = "";
  icon = "\u25a3";
  title = "";
}
</script>

<Card>
  <div class="border-b border-border px-4 py-3">
    <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
      Pages
    </p>
    <h2 class="mt-1 text-lg font-semibold text-foreground">New page</h2>
  </div>

  <form
    class="grid gap-3 p-4"
    onsubmit={(event) => {
      event.preventDefault();
      createPage();
    }}
  >
    <div>
      <Label class="mb-1">Page title</Label>
      <Input
        bind:value={title}
        placeholder="Spec, decision, launch plan"
      />
    </div>

    <div>
      <Label class="mb-1">Page icon</Label>
      <Input
        bind:value={icon}
        class="h-9"
        placeholder="\u25a3"
      />
    </div>

    <div>
      <Label class="mb-1">Page content</Label>
      <Textarea
        bind:value={content}
        class="min-h-32"
        placeholder="Write notes, specs, decisions, or project context."
      />
    </div>

    <Button
      disabled={creating || !title.trim()}
      type="submit"
    >
      {creating ? "Creating\u2026" : "Create page"}
    </Button>
  </form>
</Card>
