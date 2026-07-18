<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import type { ProjectModule } from "$lib/routes";

type ModuleCopy = {
  cta: string;
  description: string;
  eyebrow: string;
  title: string;
};

let {
  module,
  projectName,
}: {
  module: Exclude<ProjectModule, "issues" | "tickets">;
  projectName: string;
} = $props();

const copy: Record<Exclude<ProjectModule, "issues" | "tickets">, ModuleCopy> = {
  sprints: {
    cta: "Create sprint",
    description:
      "Plan time-boxed work, track progress, and keep active scope visible.",
    eyebrow: "Sprints",
    title: "Sprints are next in the project loop.",
  },
  intake: {
    cta: "Add intake item",
    description:
      "Capture incoming requests before accepting them into real project issues.",
    eyebrow: "Intake",
    title: "Triage draft work before it reaches the board.",
  },
  modules: {
    cta: "Create module",
    description:
      "Group related issues by product area, milestone, or implementation slice.",
    eyebrow: "Modules",
    title: "Modules will organize project delivery.",
  },
  pages: {
    cta: "New page",
    description:
      "Write lightweight project notes, specs, and decisions beside the work.",
    eyebrow: "Pages",
    title: "Pages will keep project context close.",
  },
  views: {
    cta: "Save view",
    description:
      "Persist filtered issue layouts for focused team workflows and reviews.",
    eyebrow: "Views",
    title: "Views will save your preferred issue slices.",
  },
};

const moduleCopy = $derived(copy[module]);
</script>

<Card
  class="grid min-h-[420px] place-items-center p-6 text-center"
>
  <div class="max-w-lg">
    <p class="text-xs font-medium uppercase tracking-[0.18em] text-primary">
      {moduleCopy.eyebrow}
    </p>
    <h2 class="mt-3 text-2xl font-semibold tracking-tight text-foreground">
      {moduleCopy.title}
    </h2>
    <p class="mt-3 text-sm leading-6 text-muted-foreground">
      {moduleCopy.description}
    </p>
    <p class="mt-2 text-xs text-muted-foreground">
      Project: {projectName}
    </p>
    <Button
      variant="outline"
      class="mt-6"
      disabled
    >
      {moduleCopy.cta} \u00b7 coming soon
    </Button>
  </div>
</Card>
