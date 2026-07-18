<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import {
  Content as DialogContent,
  Footer as DialogFooter,
  Header as DialogHeader,
  Root as DialogRoot,
  Title as DialogTitle,
} from "@workspace/ui/components/dialog";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { Textarea } from "@workspace/ui/components/textarea";

let {
  creatingProject = false,
  onClose,
  onCreateProject,
  workspaceSlug,
}: {
  creatingProject?: boolean;
  onClose: () => void;
  onCreateProject: (input: {
    description?: string;
    key: string;
    name: string;
  }) => Promise<boolean>;
  workspaceSlug: string;
} = $props();

let name = $state("");
let identifier = $state("");
let description = $state("");
let error = $state("");
let autoSyncIdentifier = $state(true);
let submitting = $state(false);

function deriveIdentifier(value: string) {
  return value
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "")
    .slice(0, 10);
}

function handleNameInput(e: Event) {
  const value = (e.target as HTMLInputElement).value;
  name = value;
  if (autoSyncIdentifier) {
    identifier = deriveIdentifier(value);
  }
}

function handleIdentifierInput(e: Event) {
  const value = (e.target as HTMLInputElement).value;
  autoSyncIdentifier = false;
  identifier = value
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "")
    .slice(0, 10);
}

async function handleSubmit(e: SubmitEvent) {
  e.preventDefault();
  error = "";

  if (!name.trim()) {
    error = "Project name is required.";
    return;
  }

  if (!identifier.trim()) {
    error = "Project identifier is required.";
    return;
  }

  submitting = true;

  try {
    const created = await onCreateProject({
      description: description.trim() || undefined,
      key: identifier.trim().toUpperCase(),
      name: name.trim(),
    });

    if (!created) {
      error = "Could not create project. Try again in a moment.";
    }
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Could not create project. Try again in a moment.";
  } finally {
    submitting = false;
  }
}
</script>

<DialogRoot
  open={true}
  onOpenChange={(open) => {
    if (!open) onClose();
  }}
>
  <DialogContent class="sm:max-w-lg">
    <DialogHeader>
      <DialogTitle>Create project</DialogTitle>
    </DialogHeader>

    <form class="space-y-5" onsubmit={handleSubmit}>
      <div class="grid grid-cols-1 gap-x-4 gap-y-5 md:grid-cols-4">
        <div class="md:col-span-3">
          <Label for="project-name" class="mb-1.5">
            Project name
            <span class="text-destructive">*</span>
          </Label>
          <!-- svelte-ignore a11y_autofocus -->
          <Input
            autofocus
            id="project-name"
            maxlength={255}
            placeholder="e.g. Website Redesign"
            required
            value={name}
            oninput={handleNameInput}
          />
        </div>

        <div>
          <Label for="project-identifier" class="mb-1.5">
            Identifier
            <span class="text-destructive">*</span>
          </Label>
          <Input
            class="font-mono uppercase"
            id="project-identifier"
            maxlength={10}
            minlength={1}
            placeholder={deriveIdentifier(name) || "KEY"}
            required
            value={identifier}
            oninput={handleIdentifierInput}
          />
        </div>
      </div>

      <p class="-mt-3 text-xs text-muted-foreground">
        Used in issue IDs like <span class="font-mono">{identifier || "KEY"}-123</span>.
        {autoSyncIdentifier ? " Auto-filled from the project name." : ""}
      </p>

      <div>
        <Label for="project-description" class="mb-1.5">Description</Label>
        <Textarea
          id="project-description"
          placeholder="What's this project about?"
          class="min-h-24 resize-none"
          bind:value={description}
        />
      </div>

      <div aria-live="polite" class="min-h-5">
        {#if error}
          <p class="text-sm text-destructive" role="alert">{error}</p>
        {/if}
      </div>

      <DialogFooter>
        <Button variant="outline" onclick={onClose}>
          Cancel
        </Button>
        <Button
          data-testid="create-project-submit"
          disabled={submitting || creatingProject || !name.trim() || !identifier.trim()}
          type="submit"
        >
          {#if submitting || creatingProject}
            <span
              aria-hidden="true"
              class="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent"
            ></span>
          {/if}
          {submitting || creatingProject ? "Creating\u2026" : "Create project"}
        </Button>
      </DialogFooter>
    </form>
  </DialogContent>
</DialogRoot>
