<script lang="ts">
import { api } from "@workspace/convex/api";
import { Button } from "@workspace/ui/components/button";
import { useMutation } from "convex-svelte";
import type { Id } from "../../../../../../convex/convex/_generated/dataModel";

type ImageType = "cover" | "logo";

let {
  projectId,
  onSelect,
  type,
}: {
  projectId?: string;
  onSelect: (url: string | null) => void;
  type: ImageType;
} = $props();

const generateUploadUrl = useMutation(api.mutations.projects.generateUploadUrl);
const storeProjectImage = useMutation(api.mutations.projects.storeProjectImage);

let uploadFile = $state<File | null>(null);
let uploadPreview = $state<string | null>(null);
let uploading = $state(false);
let uploadError = $state("");
let dragOver = $state(false);

function handleFileSelect(file: File) {
  if (!file.type.startsWith("image/")) {
    uploadError = "Please select an image file.";
    return;
  }
  uploadError = "";
  uploadFile = file;
  uploadPreview = URL.createObjectURL(file);
}

function handleFileInput(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (file) {
    handleFileSelect(file);
  }
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  dragOver = false;
  const file = e.dataTransfer?.files?.[0];
  if (file) {
    handleFileSelect(file);
  }
}

function handleDragOver(e: DragEvent) {
  e.preventDefault();
  dragOver = true;
}

function handleDragLeave() {
  dragOver = false;
}

async function handleUpload() {
  if (!uploadFile) {
    return;
  }
  uploading = true;
  uploadError = "";
  try {
    if (projectId) {
      const uploadUrl = await generateUploadUrl({});
      const response = await fetch(uploadUrl, {
        body: uploadFile,
        headers: { "Content-Type": uploadFile.type },
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("Upload failed");
      }
      const { storageId } = await response.json();
      const url = await storeProjectImage({
        projectId: projectId as Id<"projects">,
        storageId,
        type,
      });
      onSelect(url ?? null);
    } else {
      const dataUrl = await readFileAsDataUrl(uploadFile);
      onSelect(dataUrl);
    }
  } catch {
    uploadError = "Upload failed. Please try again.";
  } finally {
    uploading = false;
  }
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}
</script>

<div class="space-y-3">
  <div
    class="relative flex flex-col items-center justify-center gap-2 rounded-lg border-2 border-dashed py-8 transition {dragOver
      ? 'border-primary bg-primary/5'
      : 'border-border bg-card hover:border-input'}"
    ondragenter={handleDragOver}
    ondragleave={handleDragLeave}
    ondragover={handleDragOver}
    ondrop={handleDrop}
    role="button"
    tabindex="0"
  >
    {#if uploadPreview}
      <img
        alt="Upload preview"
        class="h-24 w-24 rounded-lg border border-border object-cover"
        src={uploadPreview}
      />
    {:else}
      <p class="text-sm text-muted-foreground">Drag and drop an image here</p>
      <span class="text-xs text-muted-foreground">or</span>
    {/if}
    <label
      class="inline-flex h-9 cursor-pointer items-center rounded-md border border-border px-4 text-xs text-muted-foreground transition hover:bg-accent hover:text-accent-foreground"
    >
      <span>Choose file</span>
      <input
        accept="image/*"
        class="sr-only"
        onchange={handleFileInput}
        type="file"
      />
    </label>
  </div>

  {#if uploadError}
    <p class="text-sm text-destructive">{uploadError}</p>
  {/if}

  {#if uploadFile}
    <Button
      class="w-full"
      disabled={uploading}
      onclick={handleUpload}
    >
      {#if uploading}
        <span
          aria-hidden="true"
          class="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent"
        ></span>
      {/if}
      {uploading ? "Uploading\u2026" : "Upload"}
    </Button>
  {/if}
</div>
