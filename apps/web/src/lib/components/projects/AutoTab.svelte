<script lang="ts">
import { Button } from "@workspace/ui/components/button";

let {
  projectName = "",
  onSelect,
}: {
  projectName?: string;
  onSelect: (url: string) => void;
} = $props();

function generateInitialLogo(text: string): string {
  const letter = text.trim().charAt(0).toUpperCase();
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128"><rect width="128" height="128" fill="black"/><text x="64" y="84" font-family="system-ui, sans-serif" font-size="72" font-weight="600" fill="white" text-anchor="middle">${letter}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const autoLogoUrl = $derived(
  projectName.trim() ? generateInitialLogo(projectName) : null
);

function handleSelectAuto() {
  if (autoLogoUrl) {
    onSelect(autoLogoUrl);
  }
}
</script>

<div class="flex flex-col items-center gap-4 py-4">
  {#if autoLogoUrl}
    <img
      alt="Generated logo"
      class="h-24 w-24 rounded-xl border border-border object-cover"
      src={autoLogoUrl}
    />
    <Button class="h-9 px-4 text-xs font-semibold" onclick={handleSelectAuto}>
      Use this
    </Button>
  {:else}
    <p class="text-sm text-muted-foreground">Enter a project name to generate a logo.</p>
  {/if}
</div>
