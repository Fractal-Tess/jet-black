<script lang="ts" module>
import type { HTMLButtonAttributes } from "svelte/elements";
import { tv, type VariantProps } from "tailwind-variants";
import { cn } from "../../utils";

export const buttonVariants = tv({
  base: "inline-flex min-h-9 shrink-0 items-center justify-center gap-2 rounded-md border border-transparent px-3 font-semibold text-xs outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
  variants: {
    variant: {
      default:
        "bg-primary text-primary-foreground shadow-[0_6px_22px_rgba(92,225,230,0.14)] hover:bg-[#7aedf1]",
      destructive:
        "border-destructive/40 bg-destructive/10 text-destructive hover:bg-destructive/20",
      ghost: "text-muted-foreground hover:bg-muted hover:text-foreground",
      outline:
        "border-input bg-secondary text-secondary-foreground hover:border-primary hover:text-primary",
      secondary: "bg-secondary text-secondary-foreground hover:bg-muted",
    },
    size: {
      default: "h-10 px-4",
      sm: "h-8 min-h-8 px-3 text-[0.7rem]",
      icon: "size-9 min-h-9 p-0",
    },
  },
  defaultVariants: {
    variant: "default",
    size: "default",
  },
});

type Props = HTMLButtonAttributes & {
  variant?: VariantProps<typeof buttonVariants>["variant"];
  size?: VariantProps<typeof buttonVariants>["size"];
};
</script>

<script lang="ts">
let {
  class: className,
  variant = "default",
  size = "default",
  type = "button",
  children,
  ...rest
}: Props = $props();
</script>

<button
  class={cn(buttonVariants({ variant, size }), className)}
  data-slot="button"
  {type}
  {...rest}
>
  {@render children?.()}
</button>
