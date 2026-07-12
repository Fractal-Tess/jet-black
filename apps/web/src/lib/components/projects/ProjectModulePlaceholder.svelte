<script lang="ts">
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
  module: Exclude<ProjectModule, "issues">;
  projectName: string;
} = $props();

const copy: Record<Exclude<ProjectModule, "issues">, ModuleCopy> = {
  cycles: {
    cta: "Create cycle",
    description:
      "Plan time-boxed work, track progress, and keep active scope visible.",
    eyebrow: "Cycles",
    title: "Cycles are next in the project loop.",
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

<section
  class="grid min-h-[420px] place-items-center rounded-xl border border-white/10 bg-[#151616] p-6 text-center"
>
  <div class="max-w-lg">
    <p class="text-xs font-medium uppercase tracking-[0.18em] text-amber-400">
      {moduleCopy.eyebrow}
    </p>
    <h2 class="mt-3 text-2xl font-semibold tracking-tight text-zinc-100">
      {moduleCopy.title}
    </h2>
    <p class="mt-3 text-sm leading-6 text-zinc-500">
      {moduleCopy.description}
    </p>
    <p class="mt-2 text-xs text-zinc-700">
      Project: {projectName}
    </p>
    <button
      class="mt-6 h-9 rounded-md border border-white/10 bg-white/[0.03] px-4 text-xs font-semibold text-zinc-400"
      disabled
      type="button"
    >
      {moduleCopy.cta} · coming soon
    </button>
  </div>
</section>
