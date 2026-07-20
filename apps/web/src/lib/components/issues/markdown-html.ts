import { Editor } from "@tiptap/core";
import Image from "@tiptap/extension-image";
import Link from "@tiptap/extension-link";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "tiptap-markdown";

let renderer: Editor | null = null;

function getRenderer(): Editor | null {
  if (typeof document === "undefined") {
    return null;
  }

  if (!renderer) {
    renderer = new Editor({
      editable: false,
      element: document.createElement("div"),
      extensions: [
        StarterKit,
        Link.configure({
          openOnClick: false,
          protocols: ["http", "https", "mailto"],
        }),
        Image,
        Markdown.configure({ html: false }),
      ],
    });
  }

  return renderer;
}

/**
 * Converts a markdown string to HTML through a headless TipTap editor. The
 * editor schema whitelists nodes/marks (raw HTML is never parsed), so the
 * output is safe to render with `{@html}`.
 */
export function renderMarkdownToHtml(markdown: string): string {
  const editor = getRenderer();

  if (!editor) {
    return "";
  }

  editor.commands.setContent(markdown, false);

  return editor.getHTML();
}
