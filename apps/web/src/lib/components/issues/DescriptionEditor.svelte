<script lang="ts">
import { Editor } from "@tiptap/core";
import Image from "@tiptap/extension-image";
import Link from "@tiptap/extension-link";
import Placeholder from "@tiptap/extension-placeholder";
import type { EditorView } from "@tiptap/pm/view";
import StarterKit from "@tiptap/starter-kit";
import { onMount } from "svelte";
import { Markdown } from "tiptap-markdown";

let {
  ariaLabel,
  class: className = "",
  onBlur,
  onUpdate,
  placeholder = "Add a description…",
  uploadFile,
  value = "",
}: {
  ariaLabel: string;
  class?: string;
  onBlur?: (markdown: string) => void;
  onUpdate?: (markdown: string) => void;
  placeholder?: string;
  uploadFile?: (file: File) => Promise<string>;
  value?: string;
} = $props();

let element = $state<HTMLDivElement | null>(null);
let editor: Editor | null = null;
let pendingUploads = $state(0);
let uploadError = $state("");

export function getMarkdown(): string {
  return editor?.storage.markdown.getMarkdown() ?? "";
}

export function setMarkdown(markdown: string) {
  editor?.commands.setContent(markdown, false);
}

async function insertFile(file: File, position: number) {
  if (!uploadFile) {
    return;
  }

  pendingUploads += 1;
  uploadError = "";

  try {
    const url = await uploadFile(file);

    if (!editor) {
      return;
    }

    const insertAt = Math.min(position, editor.state.doc.content.size);

    if (file.type.startsWith("image/")) {
      editor
        .chain()
        .insertContentAt(insertAt, {
          attrs: { alt: file.name, src: url },
          type: "image",
        })
        .run();
    } else {
      editor
        .chain()
        .insertContentAt(insertAt, {
          marks: [{ attrs: { href: url }, type: "link" }],
          text: file.name || "attachment",
          type: "text",
        })
        .run();
    }
  } catch {
    uploadError = "Upload failed. Please try again.";
  } finally {
    pendingUploads -= 1;
  }
}

function insertFiles(files: FileList, position: number) {
  for (const file of files) {
    insertFile(file, position);
  }
}

function handlePaste(view: EditorView, event: ClipboardEvent): boolean {
  const files = event.clipboardData?.files;

  if (!(uploadFile && files?.length)) {
    return false;
  }

  event.preventDefault();
  insertFiles(files, view.state.selection.from);

  return true;
}

function handleDrop(
  view: EditorView,
  event: DragEvent,
  _slice: unknown,
  moved: boolean
): boolean {
  const files = event.dataTransfer?.files;

  if (moved || !(uploadFile && files?.length)) {
    return false;
  }

  event.preventDefault();

  const coords = view.posAtCoords({
    left: event.clientX,
    top: event.clientY,
  });

  insertFiles(files, coords?.pos ?? view.state.selection.from);

  return true;
}

onMount(() => {
  if (!element) {
    return;
  }

  editor = new Editor({
    content: value,
    editorProps: {
      attributes: {
        "aria-label": ariaLabel,
        "aria-multiline": "true",
        class: "tiptap min-h-[inherit] outline-none",
        role: "textbox",
      },
      handleDrop,
      handlePaste,
    },
    element,
    extensions: [
      StarterKit,
      Link.configure({ openOnClick: false }),
      Image,
      Placeholder.configure({ placeholder }),
      Markdown.configure({
        html: false,
        transformCopiedText: true,
        transformPastedText: true,
      }),
    ],
    onBlur: () => onBlur?.(getMarkdown()),
    onUpdate: () => onUpdate?.(getMarkdown()),
  });

  return () => {
    editor?.destroy();
    editor = null;
  };
});

// Sync external value changes (e.g. realtime updates from another client)
// without clobbering in-progress typing.
$effect(() => {
  const next = value;

  if (editor && !editor.isFocused && next !== getMarkdown()) {
    editor.commands.setContent(next, false);
  }
});
</script>

<div bind:this={element} class={className}></div>
{#if pendingUploads > 0}
  <p class="mt-1 text-xs text-muted-foreground">Uploading…</p>
{/if}
{#if uploadError}
  <p class="mt-1 text-xs text-destructive">{uploadError}</p>
{/if}
