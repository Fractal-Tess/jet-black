<script lang="ts">
import type {
  HTMLAnchorAttributes,
  HTMLButtonAttributes,
} from "svelte/elements";
import { cn } from "../../utils.js";

type Props = HTMLButtonAttributes &
  HTMLAnchorAttributes & {
    variant?: "default" | "outline" | "ghost";
  };
let {
  class: className,
  variant = "default",
  href,
  type = "button",
  children,
  ...props
}: Props = $props();
const variants = {
  default: "bg-primary text-primary-foreground hover:bg-primary/90",
  outline: "border bg-background hover:bg-accent",
  ghost: "hover:bg-accent",
};
</script>

{#if href}
  <a class={cn("inline-flex h-10 items-center justify-center rounded-full px-5 text-sm font-medium transition", variants[variant], className)} {href} {...props}>{@render children?.()}</a>
{:else}
  <button class={cn("inline-flex h-10 items-center justify-center rounded-full px-5 text-sm font-medium transition disabled:opacity-50", variants[variant], className)} {type} {...props}>{@render children?.()}</button>
{/if}
