<script lang="ts">
import { type ChartConfig, THEMES } from "./chart-utils.js";

let { id, config }: { id: string; config: ChartConfig } = $props();

const colorConfig = $derived(
  config
    ? Object.entries(config).filter(
        ([, config]) => config.theme || config.color
      )
    : null
);

const themeContents = $derived.by(() => {
  if (!colorConfig?.length) {
    return;
  }

  const contents: string[] = [];
  for (const [_theme, prefix] of Object.entries(THEMES)) {
    let content = `${prefix} [data-chart=${id}] {\n`;
    const color = colorConfig.map(([key, itemConfig]) => {
      const theme = _theme as keyof typeof itemConfig.theme;
      const itemColor = itemConfig.theme?.[theme] || itemConfig.color;
      return itemColor ? `\t--color-${key}: ${itemColor};` : null;
    });

    content += `${color.join("\n")}\n}`;

    contents.push(content);
  }

  return contents.join("\n");
});
</script>

{#if themeContents}
	{#key id}
		<svelte:element this={"style"}>
			{themeContents}
		</svelte:element>
	{/key}
{/if}
