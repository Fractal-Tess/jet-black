<script lang="ts">
import {
  type ChartConfig,
  ChartContainer,
} from "@workspace/ui/components/chart";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@workspace/ui/components/tabs";
import { scaleBand } from "d3-scale";
import { BarChart } from "layerchart";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import CircleDot from "lucide-svelte/icons/circle-dot";
import Clock from "lucide-svelte/icons/clock";
import ListChecks from "lucide-svelte/icons/list-checks";
import TriangleAlert from "lucide-svelte/icons/triangle-alert";
import PriorityIcon from "$lib/components/issues/PriorityIcon.svelte";
import StateTypeIcon from "$lib/components/issues/StateTypeIcon.svelte";
import type {
  DashboardOverview,
  WorkspaceMember,
  WorkspaceUser,
} from "$lib/components/issues/types";
import { issueHref, projectModuleHref } from "$lib/routes";

let {
  loading,
  members,
  overview,
  user,
  workspaceName,
  workspaceSlug,
}: {
  loading: boolean;
  members: WorkspaceMember[];
  overview: DashboardOverview | null;
  user: WorkspaceUser;
  workspaceName: string;
  workspaceSlug: string;
} = $props();

const MORNING_END_HOUR = 12;
const EVENING_START_HOUR = 18;
const MINUTE_MS = 60_000;
const HOUR_MS = 3_600_000;
const DAY_MS = 86_400_000;
const WHITESPACE_PATTERN = /\s+/;

const greeting = $derived.by(() => {
  const hour = new Date().getHours();

  if (hour < MORNING_END_HOUR) {
    return "Good morning";
  }

  if (hour < EVENING_START_HOUR) {
    return "Good afternoon";
  }

  return "Good evening";
});

const today = new Date().toLocaleDateString(undefined, {
  day: "numeric",
  month: "long",
  weekday: "long",
});

const firstName = $derived(
  (user.name || user.email).split(WHITESPACE_PATTERN)[0]
);

const statCards = $derived(
  overview
    ? [
        {
          icon: CircleDot,
          iconClass: "text-blue-400",
          label: "Open work items",
          value: overview.stats.openIssues,
        },
        {
          icon: Clock,
          iconClass: "text-amber-400",
          label: "In progress",
          value: overview.stats.startedIssues,
        },
        {
          icon: ListChecks,
          iconClass: "text-emerald-400",
          label: "Completed this week",
          value: overview.stats.completedThisWeek,
        },
        {
          icon: TriangleAlert,
          iconClass:
            overview.stats.overdueIssues > 0 ? "text-red-400" : "text-zinc-500",
          label: "Overdue",
          value: overview.stats.overdueIssues,
        },
      ]
    : []
);

function initials(name: string) {
  const parts = name.trim().split(WHITESPACE_PATTERN);
  const first = parts[0]?.[0] ?? "?";
  const last = parts.length > 1 ? parts.at(-1)?.[0] : "";

  return `${first}${last ?? ""}`.toUpperCase();
}

function relativeTime(timestamp: number) {
  const elapsed = Date.now() - timestamp;

  if (elapsed < MINUTE_MS) {
    return "just now";
  }

  if (elapsed < HOUR_MS) {
    return `${Math.floor(elapsed / MINUTE_MS)}m ago`;
  }

  if (elapsed < DAY_MS) {
    return `${Math.floor(elapsed / HOUR_MS)}h ago`;
  }

  return `${Math.floor(elapsed / DAY_MS)}d ago`;
}

function formatDate(date: string) {
  return new Date(date).toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
  });
}

function progressPercent(project: DashboardOverview["projects"][number]) {
  if (project.totalIssues === 0) {
    return 0;
  }

  return Math.round((project.completedIssues / project.totalIssues) * 100);
}

const CREATED_COLOR = "#60a5fa";
const COMPLETED_COLOR = "#34d399";

const chartConfig = {
  completed: { color: COMPLETED_COLOR, label: "Completed" },
  created: { color: CREATED_COLOR, label: "Created" },
} satisfies ChartConfig;

const trendData = $derived(
  (overview?.trend ?? []).map((day) => ({
    ...day,
    label: new Date(day.date).toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
    }),
  }))
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

const velocityCards = $derived(
  overview
    ? [
        {
          hint: "From creation to done",
          label: "Avg cycle time",
          value: formatCycleTime(overview.stats.avgCycleTimeMs),
        },
        {
          hint: "Last 7 days",
          label: "Created this week",
          value: String(overview.stats.createdThisWeek),
        },
        {
          hint: "Last 7 days",
          label: "Completed this week",
          value: String(overview.stats.completedThisWeek),
        },
        {
          hint: "Completed minus created",
          label: "Net flow",
          value: String(
            overview.stats.completedThisWeek - overview.stats.createdThisWeek
          ),
        },
      ]
    : []
);
</script>

{#snippet issueRow(issue: DashboardOverview["recentIssues"][number])}
  <a
    class="flex min-h-10 items-center gap-3 rounded-md px-3 py-1.5 transition hover:bg-accent"
    href={issueHref({
      issueId: issue._id,
      projectId: issue.projectId,
      workspaceSlug,
    })}
  >
    {#if issue.state}
      <StateTypeIcon
        class="size-3.5 shrink-0"
        color={issue.state.color}
        type={issue.state.type}
      />
    {/if}
    <span class="w-16 shrink-0 font-mono text-[11px] text-muted-foreground">
      {issue.identifier}
    </span>
    <span class="min-w-0 flex-1 truncate text-[13px] text-foreground">
      {issue.title}
    </span>
    <PriorityIcon class="size-3.5 shrink-0" priority={issue.priority} />
    {#if issue.targetDate}
      <span
        class="flex shrink-0 items-center gap-1 text-[11px] {issue.overdue
          ? 'text-red-400'
          : 'text-muted-foreground'}"
      >
        <CalendarDays class="size-3" />
        {formatDate(issue.targetDate)}
      </span>
    {/if}
  </a>
{/snippet}

<section
  aria-label="Workspace dashboard"
  class="mx-auto flex h-[calc(100svh-5rem)] max-w-6xl flex-col gap-5"
>
  <header>
    <p class="text-xs text-muted-foreground">{today}</p>
    <h1 class="mt-1 text-xl font-semibold text-foreground">
      {greeting}, {firstName}
    </h1>
    <p class="mt-1 text-sm text-muted-foreground">
      Here's what's happening in {workspaceName}.
    </p>
  </header>

  {#if loading && !overview}
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {#each Array.from({ length: 4 }, (_, i) => i) as key (key)}
        <div
          class="h-20 animate-pulse rounded-lg border border-border bg-card/40"
        ></div>
      {/each}
    </div>
  {:else if overview}
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {#each statCards as card (card.label)}
        <div class="rounded-lg border border-border bg-card/40 p-4">
          <div class="flex items-center gap-2">
            <card.icon class="size-4 {card.iconClass}" />
            <span class="text-xs text-muted-foreground">{card.label}</span>
          </div>
          <p class="mt-2 text-2xl font-semibold tabular-nums text-foreground">
            {card.value}
          </p>
        </div>
      {/each}
    </div>

    <div class="grid min-h-0 flex-1 gap-5 lg:grid-cols-3">
      <Tabs
        class="flex min-h-0 flex-col gap-3 lg:col-span-2"
        value="overview"
      >
        <TabsList class="w-fit shrink-0">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="analytics">Analytics</TabsTrigger>
        </TabsList>

        <TabsContent
          class="flex min-h-0 flex-1 flex-col gap-5 data-[state=inactive]:hidden"
          value="overview"
        >
          <section
            aria-label="Assigned to you"
            class="flex min-h-0 flex-1 flex-col rounded-lg border border-border bg-card/40"
          >
            <h2
              class="shrink-0 border-b border-border px-4 py-3 text-sm font-medium text-foreground"
            >
              Assigned to you
            </h2>
            <div class="min-h-0 flex-1 overflow-y-auto p-1.5">
              {#each overview.myIssues as issue (issue._id)}
                {@render issueRow(issue)}
              {:else}
                <p class="px-3 py-6 text-center text-xs text-muted-foreground">
                  No open work items assigned to you. Enjoy the calm.
                </p>
              {/each}
            </div>
          </section>

          <section
            aria-label="Recently created"
            class="flex min-h-0 flex-1 flex-col rounded-lg border border-border bg-card/40"
          >
            <h2
              class="shrink-0 border-b border-border px-4 py-3 text-sm font-medium text-foreground"
            >
              Recently created
            </h2>
            <div class="min-h-0 flex-1 overflow-y-auto p-1.5">
              {#each overview.recentIssues as issue (issue._id)}
                {@render issueRow(issue)}
              {:else}
                <p class="px-3 py-6 text-center text-xs text-muted-foreground">
                  No work items yet.
                </p>
              {/each}
            </div>
          </section>
        </TabsContent>

        <TabsContent
          class="flex min-h-0 flex-1 flex-col gap-5 data-[state=inactive]:hidden"
          value="analytics"
        >
          <div class="grid shrink-0 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {#each velocityCards as card (card.label)}
              <div class="rounded-lg border border-border bg-card/40 p-4">
                <p class="text-xs text-muted-foreground">{card.label}</p>
                <p
                  class="mt-2 text-2xl font-semibold tabular-nums text-foreground"
                >
                  {card.value}
                </p>
                <p class="mt-1 text-[11px] text-muted-foreground/70">
                  {card.hint}
                </p>
              </div>
            {/each}
          </div>

          <section
            aria-label="Created vs completed"
            class="flex min-h-0 flex-1 flex-col rounded-lg border border-border bg-card/40"
          >
            <h2
              class="shrink-0 border-b border-border px-4 py-3 text-sm font-medium text-foreground"
            >
              Created vs completed
              <span class="ml-1 text-xs font-normal text-muted-foreground">
                last 14 days
              </span>
            </h2>
            <div class="min-h-0 flex-1 p-4">
              <ChartContainer class="h-full w-full" config={chartConfig}>
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
                    {
                      color: COMPLETED_COLOR,
                      key: "completed",
                      label: "Completed",
                    },
                  ]}
                  seriesLayout="group"
                  x="label"
                  xScale={scaleBand().padding(0.25)}
                />
              </ChartContainer>
            </div>
          </section>
        </TabsContent>
      </Tabs>

      <div class="flex min-h-0 flex-col gap-5 overflow-y-auto">
        <section
          aria-label="Recent activity"
          class="flex min-h-0 shrink-0 flex-col rounded-lg border border-border bg-card/40"
        >
          <h2
            class="shrink-0 border-b border-border px-4 py-3 text-sm font-medium text-foreground"
          >
            Recent activity
          </h2>
          <ul class="max-h-72 space-y-1 overflow-y-auto p-3">
            {#each overview.recentActivity as activity (activity._id)}
              <li class="flex items-start gap-2.5 rounded-md px-1 py-1.5">
                <span
                  class="mt-0.5 grid size-5 shrink-0 place-items-center rounded-full bg-amber-400/90 text-[9px] font-semibold text-black"
                >
                  {initials(activity.actorName)}
                </span>
                <div class="min-w-0 flex-1 text-xs leading-5">
                  <p class="text-muted-foreground">
                    <span class="font-medium text-foreground">
                      {activity.actorName}
                    </span>
                    {activity.message}
                    {#if activity.issueIdentifier && activity.projectId}
                      <a
                        class="font-mono text-[11px] text-primary hover:underline"
                        href={issueHref({
                          issueId: activity.issueId,
                          projectId: activity.projectId,
                          workspaceSlug,
                        })}
                      >
                        {activity.issueIdentifier}
                      </a>
                    {/if}
                  </p>
                  <p class="text-[11px] text-muted-foreground/70">
                    {relativeTime(activity.createdAt)}
                  </p>
                </div>
              </li>
            {:else}
              <li class="px-1 py-6 text-center text-xs text-muted-foreground">
                Activity will show up here as your team works.
              </li>
            {/each}
          </ul>
        </section>

        <section
          aria-label="Projects overview"
          class="rounded-lg border border-border bg-card/40"
        >
          <h2
            class="border-b border-border px-4 py-3 text-sm font-medium text-foreground"
          >
            Projects
          </h2>
          <ul class="space-y-1 p-3">
            {#each overview.projects as project (project._id)}
              <li>
                <a
                  class="block rounded-md px-2 py-2 transition hover:bg-accent"
                  href={projectModuleHref({
                    module: "tickets",
                    projectId: project._id,
                    workspaceSlug,
                  })}
                >
                  <div class="flex items-center justify-between gap-2">
                    <span class="flex min-w-0 items-center gap-2">
                      <span
                        class="size-2 shrink-0 rounded-sm bg-primary"
                        style:background-color={project.color}
                      ></span>
                      <span class="truncate text-[13px] text-foreground">
                        {project.name}
                      </span>
                    </span>
                    <span
                      class="shrink-0 text-[11px] tabular-nums text-muted-foreground"
                    >
                      {project.completedIssues}/{project.totalIssues}
                    </span>
                  </div>
                  <div
                    class="mt-1.5 h-1 overflow-hidden rounded-full bg-white/[0.06]"
                  >
                    <div
                      class="h-full rounded-full bg-emerald-400/80"
                      style:width={`${progressPercent(project)}%`}
                    ></div>
                  </div>
                </a>
              </li>
            {/each}
          </ul>
        </section>

        <section
          aria-label="Members"
          class="rounded-lg border border-border bg-card/40"
        >
          <h2
            class="border-b border-border px-4 py-3 text-sm font-medium text-foreground"
          >
            Members
            <span class="ml-1 text-xs font-normal text-muted-foreground">
              {members.length}
            </span>
          </h2>
          <ul class="space-y-0.5 p-3">
            {#each members as member (member.id)}
              <li class="flex items-center gap-2.5 rounded-md px-1 py-1.5">
                <span
                  class="grid size-6 shrink-0 place-items-center rounded-full bg-amber-400/90 text-[10px] font-semibold text-black"
                >
                  {initials(member.name)}
                </span>
                <span class="min-w-0 flex-1 truncate text-[13px] text-foreground">
                  {member.name}
                </span>
                <span class="shrink-0 text-[11px] capitalize text-muted-foreground">
                  {member.role}
                </span>
              </li>
            {/each}
          </ul>
        </section>
      </div>
    </div>
  {/if}
</section>
