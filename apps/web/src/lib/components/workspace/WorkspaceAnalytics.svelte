<script lang="ts">
import {
  type ChartConfig,
  ChartContainer,
} from "@workspace/ui/components/chart";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@workspace/ui/components/tabs";
import { scaleBand } from "d3-scale";
import { BarChart } from "layerchart";
import BriefcaseBusiness from "lucide-svelte/icons/briefcase-business";
import CheckCircle2 from "lucide-svelte/icons/check-circle-2";
import CircleDot from "lucide-svelte/icons/circle-dot";
import Clock3 from "lucide-svelte/icons/clock-3";
import FileText from "lucide-svelte/icons/file-text";
import FolderKanban from "lucide-svelte/icons/folder-kanban";
import Inbox from "lucide-svelte/icons/inbox";
import Layers3 from "lucide-svelte/icons/layers-3";
import ShieldCheck from "lucide-svelte/icons/shield-check";
import Users from "lucide-svelte/icons/users";
import type {
  Project,
  WorkspaceAnalyticsData,
} from "$lib/components/issues/types";

const ALL_PROJECTS = "all";
const DAY_MS = 86_400_000;
const HOUR_MS = 3_600_000;
const INSIGHT_COLOR = "var(--info)";
const CREATED_COLOR = "var(--info)";
const COMPLETED_COLOR = "var(--success)";

let {
  analytics,
  loading,
  onProjectChange,
  projects,
  selectedProjectId,
}: {
  analytics: WorkspaceAnalyticsData | null;
  loading: boolean;
  onProjectChange: (projectId: Project["_id"] | undefined) => void;
  projects: Project[];
  selectedProjectId?: Project["_id"];
} = $props();

let activeView = $state("overview");

const selectedProjectLabel = $derived(
  projects.find((project) => project._id === selectedProjectId)?.name ??
    "All projects"
);

const overviewCards = $derived(
  analytics
    ? [
        {
          hint: "Workspace-wide",
          icon: Users,
          label: "Total members",
          value: analytics.people.total,
        },
        {
          hint: "Workspace owners",
          icon: ShieldCheck,
          label: "Owners",
          value: analytics.people.owners,
        },
        {
          hint: analytics.scope.projectName ?? "Selected scope",
          icon: FolderKanban,
          label: "Projects",
          value: analytics.totals.projects,
        },
        {
          hint: analytics.scope.projectName ?? "Selected scope",
          icon: BriefcaseBusiness,
          label: "Work items",
          value: analytics.totals.workItems,
        },
        {
          hint: analytics.scope.projectName ?? "Selected scope",
          icon: Clock3,
          label: "Cycles",
          value: analytics.totals.cycles,
        },
        {
          hint: analytics.scope.projectName ?? "Selected scope",
          icon: Inbox,
          label: "Intake",
          value: analytics.totals.intake,
        },
        {
          hint: analytics.scope.projectName ?? "Selected scope",
          icon: Layers3,
          label: "Modules",
          value: analytics.totals.modules,
        },
        {
          hint: analytics.scope.projectName ?? "Selected scope",
          icon: FileText,
          label: "Pages",
          value: analytics.totals.pages,
        },
      ]
    : []
);

const insightData = $derived(
  analytics
    ? [
        { label: "Work items", value: analytics.totals.workItems },
        { label: "Cycles", value: analytics.totals.cycles },
        { label: "Modules", value: analytics.totals.modules },
        { label: "Intake", value: analytics.totals.intake },
        { label: "Pages", value: analytics.totals.pages },
      ]
    : []
);

const trendData = $derived(
  (analytics?.trend ?? []).map((day) => ({
    ...day,
    label: new Date(day.date).toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
    }),
  }))
);

const statusData = $derived(
  analytics
    ? [
        {
          color: "bg-muted-foreground",
          label: "Backlog",
          value: analytics.stats.statusCounts.backlog,
        },
        {
          color: "bg-secondary-foreground",
          label: "Unstarted",
          value: analytics.stats.statusCounts.unstarted,
        },
        {
          color: "bg-warning",
          label: "In progress",
          value: analytics.stats.statusCounts.started,
        },
        {
          color: "bg-success",
          label: "Completed",
          value: analytics.stats.statusCounts.completed,
        },
        {
          color: "bg-destructive",
          label: "Cancelled",
          value: analytics.stats.statusCounts.cancelled,
        },
      ]
    : []
);

const velocityCards = $derived(
  analytics
    ? [
        {
          hint: "Creation to completion",
          icon: Clock3,
          label: "Avg cycle time",
          value: formatCycleTime(analytics.stats.avgCycleTimeMs),
        },
        {
          hint: "Last 7 days",
          icon: CircleDot,
          label: "Created",
          value: String(analytics.stats.createdInPeriod),
        },
        {
          hint: "Last 7 days",
          icon: CheckCircle2,
          label: "Completed",
          value: String(analytics.stats.completedInPeriod),
        },
        {
          hint: "Completed minus created",
          icon: BriefcaseBusiness,
          label: "Net flow",
          value: formatSignedNumber(analytics.stats.netFlow),
        },
      ]
    : []
);

const insightChartConfig = {
  value: { color: INSIGHT_COLOR, label: "Total" },
} satisfies ChartConfig;

const trendChartConfig = {
  completed: { color: COMPLETED_COLOR, label: "Completed" },
  created: { color: CREATED_COLOR, label: "Created" },
} satisfies ChartConfig;

const totalStatusItems = $derived(
  statusData.reduce((total, status) => total + status.value, 0)
);

function formatCycleTime(ms: number | null) {
  if (ms === null) {
    return "—";
  }

  if (ms < HOUR_MS) {
    return "<1h";
  }

  if (ms < DAY_MS) {
    return `${Math.round(ms / HOUR_MS)}h`;
  }

  return `${(ms / DAY_MS).toFixed(1)}d`;
}

function formatSignedNumber(value: number) {
  return value > 0 ? `+${value}` : String(value);
}

function statusPercent(value: number) {
  return totalStatusItems === 0
    ? 0
    : Math.round((value / totalStatusItems) * 100);
}

function handleProjectChange(value: string) {
  onProjectChange(
    value === ALL_PROJECTS ? undefined : (value as Project["_id"])
  );
}
</script>

<section aria-labelledby="analytics-title" class="mx-auto max-w-6xl space-y-6">
  <header class="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
    <div>
      <h1 id="analytics-title" class="text-xl font-semibold text-foreground">
        Analytics
      </h1>
      <p class="mt-1 text-sm text-muted-foreground">
        Understand workspace activity, delivery health, and project progress.
      </p>
    </div>

    <Select
      disabled={projects.length === 0}
      onValueChange={handleProjectChange}
      type="single"
      value={selectedProjectId ?? ALL_PROJECTS}
    >
      <SelectTrigger
        aria-label="Filter analytics by project"
        class="h-8 w-full min-w-44 justify-between border-input bg-background px-2.5 text-xs text-foreground sm:w-auto"
        size="sm"
      >
        {selectedProjectLabel}
      </SelectTrigger>
      <SelectContent class="border border-border bg-popover text-popover-foreground">
        <SelectItem label="All projects" value={ALL_PROJECTS} />
        {#each projects as project (project._id)}
          <SelectItem label={project.name} value={project._id} />
        {/each}
      </SelectContent>
    </Select>
  </header>

  <Tabs bind:value={activeView} class="gap-5">
    <TabsList class="w-fit">
      <TabsTrigger value="overview">Overview</TabsTrigger>
      <TabsTrigger value="work-items">Work items</TabsTrigger>
    </TabsList>

    {#if loading && !analytics}
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {#each Array.from({ length: 8 }, (_, index) => index) as key (key)}
          <div class="h-24 animate-pulse rounded-lg border border-border bg-card/40"></div>
        {/each}
      </div>
    {:else if analytics}
      <TabsContent class="space-y-6 data-[state=inactive]:hidden" value="overview">
        <section aria-labelledby="overview-heading">
          <div class="mb-3 flex items-baseline justify-between gap-4">
            <h2 id="overview-heading" class="text-base font-medium text-foreground">
              Overview
            </h2>
            <p class="text-meta text-muted-foreground">
              People stay workspace-wide; delivery metrics follow the project filter.
            </p>
          </div>

          <dl class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {#each overviewCards as card (card.label)}
              <div class="rounded-lg border border-border bg-card/40 p-4">
                <dt class="flex items-center gap-2 text-xs text-muted-foreground">
                  <card.icon class="size-4 text-primary" />
                  {card.label}
                </dt>
                <dd class="mt-3 text-2xl font-semibold tabular-nums text-foreground">
                  {card.value}
                </dd>
                <p class="mt-1 text-meta text-muted-foreground/70">{card.hint}</p>
              </div>
            {/each}
          </dl>
        </section>

        <section aria-labelledby="project-insights-heading" class="space-y-3">
          <h2 id="project-insights-heading" class="text-base font-medium text-foreground">
            Project insights
          </h2>

          <div class="grid gap-5 lg:grid-cols-[minmax(0,1.45fr)_minmax(18rem,0.8fr)]">
            <div class="rounded-lg border border-border bg-card/40">
              <div class="border-b border-border px-4 py-3">
                <h3 class="text-sm font-medium text-foreground">Scope distribution</h3>
                <p class="mt-0.5 text-meta text-muted-foreground">
                  Records across the selected project scope.
                </p>
              </div>
              <div class="h-72 p-4">
                <ChartContainer class="h-full w-full" config={insightChartConfig}>
                  <BarChart
                    axis="x"
                    data={insightData}
                    props={{
                      bars: { radius: 3, rounded: "top", strokeWidth: 0 },
                      xAxis: { format: (value: string) => value },
                    }}
                    series={[{ color: INSIGHT_COLOR, key: "value", label: "Total" }]}
                    x="label"
                    xScale={scaleBand().padding(0.35)}
                  />
                </ChartContainer>
              </div>
              <table class="sr-only">
                <caption>Record totals for the selected analytics scope</caption>
                <thead><tr><th>Type</th><th>Total</th></tr></thead>
                <tbody>
                  {#each insightData as insight (insight.label)}
                    <tr><td>{insight.label}</td><td>{insight.value}</td></tr>
                  {/each}
                </tbody>
              </table>
            </div>

            <div class="rounded-lg border border-border bg-card/40">
              <div class="border-b border-border px-4 py-3">
                <h3 class="text-sm font-medium text-foreground">Project progress</h3>
                <p class="mt-0.5 text-meta text-muted-foreground">
                  Completed work items across active project records.
                </p>
              </div>
              <ul class="max-h-72 space-y-1 overflow-y-auto p-3">
                {#each analytics.projects as project (project.projectId)}
                  <li class="rounded-md px-2 py-2.5 hover:bg-accent/60">
                    <div class="flex items-center gap-3">
                      <span
                        aria-hidden="true"
                        class="grid size-8 shrink-0 place-items-center rounded-md border border-border bg-muted/40 text-xs font-semibold text-foreground"
                        style:color={project.color ?? undefined}
                      >
                        {project.name.slice(0, 1).toUpperCase()}
                      </span>
                      <div class="min-w-0 flex-1">
                        <div class="flex items-center justify-between gap-3">
                          <span class="truncate text-sm text-foreground">{project.name}</span>
                          <span class="text-xs tabular-nums text-muted-foreground">{project.progress}%</span>
                        </div>
                        <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-muted">
                          <div
                            class="h-full rounded-full bg-primary"
                            style:width={`${project.progress}%`}
                          ></div>
                        </div>
                        <p class="mt-1.5 text-meta text-muted-foreground">
                          {project.completedIssues} completed · {project.openIssues} open · {project.totalIssues} total
                        </p>
                      </div>
                    </div>
                  </li>
                {:else}
                  <li class="px-2 py-8 text-center text-xs text-muted-foreground">
                    No projects in this workspace yet.
                  </li>
                {/each}
              </ul>
            </div>
          </div>
        </section>
      </TabsContent>

      <TabsContent class="space-y-6 data-[state=inactive]:hidden" value="work-items">
        <section aria-labelledby="work-item-summary-heading">
          <h2 id="work-item-summary-heading" class="mb-3 text-base font-medium text-foreground">
            Work item activity
          </h2>
          <dl class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {#each velocityCards as card (card.label)}
              <div class="rounded-lg border border-border bg-card/40 p-4">
                <dt class="flex items-center gap-2 text-xs text-muted-foreground">
                  <card.icon class="size-4 text-primary" />
                  {card.label}
                </dt>
                <dd class="mt-3 text-2xl font-semibold tabular-nums text-foreground">
                  {card.value}
                </dd>
                <p class="mt-1 text-meta text-muted-foreground/70">{card.hint}</p>
              </div>
            {/each}
          </dl>
        </section>

        <div class="grid gap-5 lg:grid-cols-[minmax(0,1.5fr)_minmax(17rem,0.7fr)]">
          <section aria-labelledby="trend-heading" class="rounded-lg border border-border bg-card/40">
            <div class="border-b border-border px-4 py-3">
              <h2 id="trend-heading" class="text-sm font-medium text-foreground">
                Created vs completed
              </h2>
              <p class="mt-0.5 text-meta text-muted-foreground">Last 14 days</p>
            </div>
            <div class="h-80 p-4">
              <ChartContainer class="h-full w-full" config={trendChartConfig}>
                <BarChart
                  axis="x"
                  data={trendData}
                  legend
                  props={{
                    bars: { radius: 2, rounded: "top", strokeWidth: 0 },
                    xAxis: { format: (value: string) => value },
                  }}
                  series={[
                    { color: CREATED_COLOR, key: "created", label: "Created" },
                    { color: COMPLETED_COLOR, key: "completed", label: "Completed" },
                  ]}
                  seriesLayout="group"
                  x="label"
                  xScale={scaleBand().padding(0.25)}
                />
              </ChartContainer>
            </div>
            <table class="sr-only">
              <caption>Created and completed work items for the last 14 days</caption>
              <thead><tr><th>Date</th><th>Created</th><th>Completed</th></tr></thead>
              <tbody>
                {#each trendData as day (day.date)}
                  <tr><td>{day.label}</td><td>{day.created}</td><td>{day.completed}</td></tr>
                {/each}
              </tbody>
            </table>
          </section>

          <section aria-labelledby="status-heading" class="rounded-lg border border-border bg-card/40">
            <div class="border-b border-border px-4 py-3">
              <h2 id="status-heading" class="text-sm font-medium text-foreground">
                Current work by status
              </h2>
              <p class="mt-0.5 text-meta text-muted-foreground">
                {analytics.stats.overdueIssues} overdue open work items
              </p>
            </div>
            <dl class="space-y-4 p-4">
              {#each statusData as status (status.label)}
                <div>
                  <div class="flex items-center justify-between gap-3 text-xs">
                    <dt class="flex items-center gap-2 text-muted-foreground">
                      <span class="size-2 rounded-full {status.color}"></span>
                      {status.label}
                    </dt>
                    <dd class="tabular-nums text-foreground">
                      {status.value} <span class="text-muted-foreground">({statusPercent(status.value)}%)</span>
                    </dd>
                  </div>
                  <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-muted">
                    <div
                      class="h-full rounded-full {status.color}"
                      style:width={`${statusPercent(status.value)}%`}
                    ></div>
                  </div>
                </div>
              {/each}
            </dl>
          </section>
        </div>
      </TabsContent>
    {/if}
  </Tabs>
</section>
