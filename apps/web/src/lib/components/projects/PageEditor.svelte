<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { Textarea } from "@workspace/ui/components/textarea";
import type { ProjectPage } from "$lib/components/issues/types";

let {
  page,
  onUpdate,
  totalPages,
}: {
  page: ProjectPage;
  onUpdate: (
    pageId: ProjectPage["_id"],
    input: {
      content?: string;
      icon?: string | null;
      title?: string;
    }
  ) => Promise<void>;
  totalPages?: number;
} = $props();

let draftContent = $state("");
let draftIcon = $state("");
let draftTitle = $state("");

$effect(() => {
  draftContent = page.content;
  draftIcon = page.icon ?? "";
  draftTitle = page.title;
});

async function saveSelectedPage() {
  await onUpdate(page._id, {
    content: draftContent,
    icon: draftIcon.trim() || null,
    title: draftTitle,
  });
}
</script>

<Card>
  <div
    class="flex items-center justify-between border-b border-border px-4 py-3"
  >
    <div>
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
        Editor
      </p>
      <h2 class="mt-1 text-lg font-semibold text-foreground">
        {page.title}
      </h2>
    </div>
    {#if totalPages !== undefined}
      <Badge variant="outline">
        {totalPages} total
      </Badge>
    {/if}
  </div>

  <div class="grid gap-4 p-4">
    <div class="grid gap-3 sm:grid-cols-[72px_minmax(0,1fr)]">
      <div>
        <Label class="mb-1">Icon</Label>
        <Input bind:value={draftIcon} />
      </div>
      <div>
        <Label class="mb-1">Title</Label>
        <Input bind:value={draftTitle} />
      </div>
    </div>

    <div>
      <Label class="mb-1">Content</Label>
      <Textarea
        bind:value={draftContent}
        class="min-h-[360px] font-mono leading-6"
      />
    </div>

    <Button
      class="justify-self-start"
      disabled={!draftTitle.trim()}
      onclick={saveSelectedPage}
    >
      Save page
    </Button>
  </div>
</Card>
