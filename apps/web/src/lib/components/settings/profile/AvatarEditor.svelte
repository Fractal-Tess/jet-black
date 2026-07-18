<script lang="ts">
import Camera from "lucide-svelte/icons/camera";
import Upload from "lucide-svelte/icons/upload";
import X from "lucide-svelte/icons/x";

let {
  currentImage,
  initials,
  onSave,
}: {
  currentImage?: string | null;
  initials: string;
  onSave: (file: File) => Promise<void>;
} = $props();

const MAX_SIZE = 5 * 1024 * 1024; // 5 MB
const ACCEPTED_TYPES = ["image/png", "image/jpeg", "image/webp"];

let fileInput: HTMLInputElement | undefined = $state();
let previewOpen = $state(false);
let previewUrl = $state<string | null>(null);
let previewFile = $state<File | null>(null);
let saving = $state(false);
let error = $state("");
let dragOver = $state(false);

function selectFile(file: File) {
  error = "";

  if (!ACCEPTED_TYPES.includes(file.type)) {
    error = "Please upload a PNG, JPEG, or WebP image.";
    return;
  }

  if (file.size > MAX_SIZE) {
    error = "Image must be smaller than 5 MB.";
    return;
  }

  if (previewUrl) {
    URL.revokeObjectURL(previewUrl);
  }

  previewFile = file;
  previewUrl = URL.createObjectURL(file);
  previewOpen = true;
}

function handleFileInput(e: Event) {
  const target = e.target as HTMLInputElement;
  const file = target.files?.[0];
  if (file) {
    selectFile(file);
  }
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  dragOver = false;
  const file = e.dataTransfer?.files[0];
  if (file) {
    selectFile(file);
  }
}

function handleDragOver(e: DragEvent) {
  e.preventDefault();
  dragOver = true;
}

async function handleSave() {
  if (!previewFile) {
    return;
  }

  saving = true;
  error = "";

  try {
    await onSave(previewFile);
    previewOpen = false;
    cleanup();
  } catch (e) {
    error = e instanceof Error ? e.message : "Upload failed.";
  } finally {
    saving = false;
  }
}

function cleanup() {
  if (previewUrl) {
    URL.revokeObjectURL(previewUrl);
  }
  previewUrl = null;
  previewFile = null;
  if (fileInput) {
    fileInput.value = "";
  }
}

function cancel() {
  previewOpen = false;
  error = "";
  cleanup();
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && previewOpen) {
    cancel();
  }
}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex flex-col items-start gap-3">
  <!-- Upload modal -->
  {#if previewOpen && previewUrl}
    <!-- Backdrop (click to close) -->
    <button
      aria-label="Close"
      class="fixed inset-0 z-50 bg-background/80"
      onclick={cancel}
      type="button"
    ></button>
    <div
      class="fixed inset-0 z-50 flex pointer-events-none items-center justify-center"
    >
      <div
        class="pointer-events-auto flex w-full max-w-sm flex-col items-center gap-5 rounded-xl border border-border bg-card p-6 shadow-2xl"
      >
        <div class="flex w-full items-center justify-between">
          <h3 class="text-sm font-medium text-foreground">Upload photo</h3>
          <button
            aria-label="Cancel"
            class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground"
            onclick={cancel}
            type="button"
          >
            <X class="size-4" />
          </button>
        </div>

        <!-- Preview -->
        <div
          class="relative size-48 overflow-hidden rounded-full border-2 border-border"
        >
          <img
            alt="Avatar preview"
            class="size-full object-cover"
            src={previewUrl}
          />
        </div>

        {#if error}
          <p class="text-xs text-destructive" role="alert">{error}</p>
        {/if}

        <div class="flex gap-2">
          <button
            class="h-9 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
            disabled={saving}
            onclick={handleSave}
            type="button"
          >
            {saving ? "Uploading…" : "Save photo"}
          </button>
          <button
            class="h-9 rounded-md border border-border px-4 text-sm text-muted-foreground transition hover:bg-accent hover:text-foreground"
            disabled={saving}
            onclick={() => fileInput?.click()}
            type="button"
          >
            Choose different
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Avatar display + hover trigger -->
  <button
    aria-label="Upload profile photo"
    class="group relative"
    onclick={() => fileInput?.click()}
    ondragover={handleDragOver}
    ondragleave={() => (dragOver = false)}
    ondrop={handleDrop}
    type="button"
  >
    {#if currentImage}
      <img
        alt=""
        class="size-20 rounded-full border-2 border-border object-cover transition {dragOver
          ? 'border-primary'
          : ''}"
        src={currentImage}
      />
    {:else}
      <div
        class="grid size-20 place-items-center rounded-full border-2 border-border bg-primary text-2xl font-bold text-primary-foreground transition {dragOver
          ? 'border-primary'
          : ''}"
      >
        {initials}
      </div>
    {/if}

    <div
      class="absolute inset-0 flex items-center justify-center rounded-full bg-background/60 opacity-0 transition group-hover:opacity-100"
    >
      <Camera class="size-6 text-foreground" />
    </div>
  </button>

  <input
    accept={ACCEPTED_TYPES.join(",")}
    bind:this={fileInput}
    class="hidden"
    onchange={handleFileInput}
    type="file"
  />

  <div class="flex items-center gap-2">
    <button
      class="flex items-center gap-1.5 text-xs text-muted-foreground transition hover:text-foreground"
      onclick={() => fileInput?.click()}
      type="button"
    >
      <Upload class="size-3" />
      {currentImage ? "Change photo" : "Upload photo"}
    </button>
  </div>

  {#if error && !previewOpen}
    <p class="text-xs text-destructive" role="alert">{error}</p>
  {/if}
</div>
