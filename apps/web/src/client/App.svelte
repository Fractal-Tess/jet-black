<script lang="ts">
import type {
  ProductCommand,
  ProductProject,
  ProductSnapshot,
  ProductTicket,
  ProductTicketPriority,
  ProductWorkspace,
} from "@workspace/shared/protocol";
import {
  Activity,
  Archive,
  BarChart3,
  Blocks,
  Bot,
  Boxes,
  Check,
  ChevronDown,
  CircleDot,
  Cloud,
  FileText,
  Gauge,
  Inbox,
  Kanban,
  Layers3,
  List,
  LogOut,
  Menu,
  PanelLeftClose,
  Plus,
  Search,
  Settings,
  Sparkles,
  Terminal,
  Users,
  Wifi,
  WifiOff,
  X,
  Zap,
} from "lucide-svelte";
import { onMount } from "svelte";
import {
  configuredInstance,
  JetBlackClient,
  JetBlackClientError,
  saveConfiguredInstance,
} from "./jet-black-client";

type View =
  | "analytics"
  | "intake"
  | "modules"
  | "pages"
  | "settings"
  | "sprints"
  | "tickets";

let client = new JetBlackClient();
let snapshot = $state<ProductSnapshot | null>(null);
let activeWorkspaceId = $state<string | null>(null);
let activeProjectId = $state<string | null>(null);
let activeView = $state<View>("tickets");
let loading = $state(true);
let submitting = $state(false);
let connected = $state(false);
let sidebarOpen = $state(false);
let createMenuOpen = $state(false);
let modal = $state<"project" | "ticket" | "workspace" | null>(null);
let errorMessage = $state("");
let loginEmail = $state("dev@jet-black.local");
let loginPassword = $state("");
let search = $state("");
let ticketLayout = $state<"board" | "list">("board");
let instanceDraft = $state(configuredInstance());
let workspaceName = $state("");
let workspaceSlug = $state("");
let projectName = $state("");
let projectIdentifier = $state("");
let repositoryIdentity = $state("");
let ticketTitle = $state("");
let ticketDescription = $state("");
let ticketPriority = $state<ProductTicketPriority>("none");
let realtimeCleanup: (() => void) | undefined;

const activeWorkspace = $derived(
  snapshot?.workspaces.find(
    (workspace) => workspace.id === activeWorkspaceId
  ) ??
    snapshot?.workspaces[0] ??
    null
);
const workspaceProjects = $derived(
  snapshot?.projects.filter(
    (project) => project.workspace_id === activeWorkspace?.id
  ) ?? []
);
const activeProject = $derived(
  workspaceProjects.find((project) => project.id === activeProjectId) ??
    workspaceProjects[0] ??
    null
);
const projectTickets = $derived(
  snapshot?.tickets.filter(
    (ticket) =>
      ticket.project_id === activeProject?.id &&
      `${activeProject.identifier}-${ticket.sequence_number} ${ticket.title}`
        .toLowerCase()
        .includes(search.toLowerCase())
  ) ?? []
);
const urgentTickets = $derived(
  projectTickets.filter(
    (ticket) => ticket.priority === "urgent" || ticket.priority === "high"
  ).length
);

onMount(() => {
  const requestedView = viewFromPath(window.location.pathname);
  activeView = requestedView;
  initialize().catch((error) => {
    errorMessage = readableError(error);
  });
  const popstate = () => {
    activeView = viewFromPath(window.location.pathname);
  };
  window.addEventListener("popstate", popstate);
  return () => {
    realtimeCleanup?.();
    window.removeEventListener("popstate", popstate);
  };
});

async function initialize(): Promise<void> {
  loading = true;
  errorMessage = "";
  try {
    const bootstrap = await client.bootstrap();
    if (bootstrap.protocol_version !== "1.0") {
      throw new Error(
        `Instance protocol ${bootstrap.protocol_version} is not supported.`
      );
    }
    await client.currentSession();
    await refreshSnapshot();
  } catch (error) {
    if (!(error instanceof JetBlackClientError && error.status === 401)) {
      errorMessage = readableError(error);
    }
  } finally {
    loading = false;
  }
}

async function login(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  submitting = true;
  errorMessage = "";
  try {
    await client.login(loginEmail, loginPassword);
    loginPassword = "";
    await refreshSnapshot();
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    submitting = false;
  }
}

async function logout(): Promise<void> {
  submitting = true;
  try {
    await client.logout();
  } catch {
    // Expired sessions are already effectively signed out.
  } finally {
    realtimeCleanup?.();
    snapshot = null;
    submitting = false;
  }
}

async function refreshSnapshot(workspaceId = activeWorkspaceId ?? undefined) {
  const next = await client.snapshot(workspaceId);
  snapshot = next;
  activeWorkspaceId =
    next.workspaces.find((workspace) => workspace.id === activeWorkspaceId)
      ?.id ??
    next.workspaces[0]?.id ??
    null;
  const availableProjects = next.projects.filter(
    (project) => project.workspace_id === activeWorkspaceId
  );
  activeProjectId =
    availableProjects.find((project) => project.id === activeProjectId)?.id ??
    availableProjects[0]?.id ??
    null;
  connectRealtime(next);
}

function connectRealtime(next: ProductSnapshot): void {
  realtimeCleanup?.();
  const workspaceId = activeWorkspaceId ?? next.workspaces[0]?.id;
  if (!workspaceId) {
    return;
  }
  realtimeCleanup = client.connect(
    workspaceId,
    next.event_cursor,
    (message) => {
      if (message.type === "events" && message.data.events.length > 0) {
        refreshSnapshot(workspaceId).catch((error) => {
          errorMessage = readableError(error);
        });
      }
    },
    (value) => {
      connected = value;
    }
  );
}

async function chooseWorkspace(workspace: ProductWorkspace): Promise<void> {
  activeWorkspaceId = workspace.id;
  activeProjectId = null;
  sidebarOpen = false;
  await refreshSnapshot(workspace.id);
}

function chooseProject(project: ProductProject): void {
  activeProjectId = project.id;
  activeView = "tickets";
  sidebarOpen = false;
  updatePath("tickets");
}

function navigate(view: View): void {
  activeView = view;
  sidebarOpen = false;
  updatePath(view);
}

function updatePath(view: View): void {
  const workspace = activeWorkspace?.slug ?? "workspace";
  const project = activeProject?.id ?? "project";
  const path =
    view === "settings"
      ? `/workspace/${workspace}/settings`
      : `/workspace/${workspace}/projects/${project}/${view}`;
  window.history.pushState({}, "", path);
}

async function submitModal(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  submitting = true;
  errorMessage = "";
  try {
    let command: ProductCommand;
    if (modal === "workspace") {
      command = {
        type: "create_workspace",
        data: { name: workspaceName, slug: workspaceSlug },
      };
    } else if (modal === "project" && activeWorkspace) {
      command = {
        type: "create_project",
        data: {
          description: "",
          identifier: projectIdentifier,
          name: projectName,
          repository_identity: repositoryIdentity || null,
          workspace_id: activeWorkspace.id,
        },
      };
    } else if (modal === "ticket" && activeProject) {
      command = {
        type: "create_ticket",
        data: {
          description: ticketDescription,
          idempotency_key: crypto.randomUUID(),
          priority: ticketPriority,
          project_id: activeProject.id,
          title: ticketTitle,
        },
      };
    } else {
      return;
    }
    const response = await client.command(command);
    if (response.type === "workspace_created") {
      activeWorkspaceId = response.data.id;
      activeProjectId = null;
    } else if (response.type === "project_created") {
      activeProjectId = response.data.id;
    }
    resetModal();
    await refreshSnapshot();
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    submitting = false;
  }
}

function resetModal(): void {
  modal = null;
  createMenuOpen = false;
  workspaceName = "";
  workspaceSlug = "";
  projectName = "";
  projectIdentifier = "";
  repositoryIdentity = "";
  ticketTitle = "";
  ticketDescription = "";
  ticketPriority = "none";
}

function saveInstance(event: SubmitEvent): void {
  event.preventDefault();
  try {
    const instance = saveConfiguredInstance(instanceDraft);
    const nextUrl = new URL(window.location.href);
    if (instance) {
      nextUrl.searchParams.set("instance", instance);
    } else {
      nextUrl.searchParams.delete("instance");
    }
    window.location.assign(nextUrl);
  } catch (error) {
    errorMessage = readableError(error);
  }
}

function readableError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return "Something went wrong.";
}

function viewFromPath(path: string): View {
  for (const view of [
    "analytics",
    "intake",
    "modules",
    "pages",
    "settings",
    "sprints",
    "tickets",
  ] as const) {
    if (path.includes(`/${view}`)) {
      return view;
    }
  }
  return "tickets";
}

function priorityLabel(priority: ProductTicketPriority): string {
  return priority === "none"
    ? "No priority"
    : priority[0].toUpperCase() + priority.slice(1);
}

function ticketKey(ticket: ProductTicket): string {
  return `${activeProject?.identifier ?? "JB"}-${ticket.sequence_number}`;
}
</script>

<svelte:head>
  <title>{activeProject ? `${activeProject.name} · Jet Black` : "Jet Black"}</title>
</svelte:head>

{#if loading}
  <main class="loading-screen" aria-label="Loading Jet Black">
    <div class="brand-orbit"><span>JB</span></div>
    <p>Connecting to control plane</p>
  </main>
{:else if !snapshot}
  <main class="auth-screen">
    <section class="auth-story">
      <div class="brand-lockup"><span class="brand-mark">JB</span> JET BLACK</div>
      <div>
        <p class="eyebrow"><Sparkles size={14} /> AGENTIC DELIVERY SYSTEM</p>
        <h1>Ship ambitious work.<br /><em>Keep control.</em></h1>
        <p class="auth-copy">
          Plan with your team, delegate to coding agents, review every change,
          and land exact revisions from one focused workspace.
        </p>
      </div>
      <div class="signal-grid" aria-hidden="true">
        <span>PLAN</span><span>EXECUTE</span><span>REVIEW</span><span>SHIP</span>
      </div>
    </section>
    <section class="auth-panel">
      <form class="auth-card" onsubmit={login}>
        <div class="mobile-brand"><span class="brand-mark">JB</span> JET BLACK</div>
        <div>
          <p class="eyebrow">WELCOME BACK</p>
          <h2>Sign in to your instance</h2>
          <p>Use the account managed by this Jet Black control plane.</p>
        </div>
        {#if errorMessage}<div class="alert" role="alert">{errorMessage}</div>{/if}
        <label>
          <span>Email</span>
          <input
            autocomplete="email"
            bind:value={loginEmail}
            name="email"
            placeholder="you@company.com"
            required
            type="email"
          />
        </label>
        <label>
          <span>Password</span>
          <input
            autocomplete="current-password"
            bind:value={loginPassword}
            minlength="12"
            name="password"
            placeholder="••••••••••••"
            required
            type="password"
          />
        </label>
        <button class="primary-button" disabled={submitting} type="submit">
          {submitting ? "Authenticating…" : "Enter workspace"}
          <Zap size={16} />
        </button>
        <p class="instance-caption">
          <Cloud size={13} />
          {client.baseUrl || "This device · local instance"}
        </p>
      </form>
    </section>
  </main>
{:else}
  <div class:sidebar-visible={sidebarOpen} class="product-shell">
    <button
      aria-label="Close navigation"
      class="sidebar-scrim"
      onclick={() => (sidebarOpen = false)}
      type="button"
    ></button>
    <aside class="sidebar">
      <div class="sidebar-brand">
        <span class="brand-mark">JB</span>
        <strong>JET BLACK</strong>
        <button
          aria-label="Close sidebar"
          class="icon-button mobile-only"
          onclick={() => (sidebarOpen = false)}
          type="button"
        ><PanelLeftClose size={17} /></button>
      </div>

      <div class="workspace-picker">
        <button
          aria-expanded={createMenuOpen}
          class="workspace-button"
          onclick={() => (createMenuOpen = !createMenuOpen)}
          type="button"
        >
          <span class="workspace-avatar">{activeWorkspace?.name.slice(0, 2).toUpperCase()}</span>
          <span><small>WORKSPACE</small><strong>{activeWorkspace?.name}</strong></span>
          <ChevronDown size={15} />
        </button>
        {#if createMenuOpen}
          <div class="workspace-menu">
            {#each snapshot.workspaces as workspace (workspace.id)}
              <button onclick={() => chooseWorkspace(workspace)} type="button">
                <span>{workspace.name}</span><small>{workspace.role}</small>
              </button>
            {/each}
            <button class="menu-create" onclick={() => (modal = "workspace")} type="button">
              <Plus size={14} /> New workspace
            </button>
          </div>
        {/if}
      </div>

      <nav aria-label="Workspace navigation">
        <p>DELIVER</p>
        <button class:active={activeView === "tickets"} onclick={() => navigate("tickets")} type="button">
          <Kanban size={16} /> Tickets <span>{projectTickets.length}</span>
        </button>
        <button class:active={activeView === "intake"} onclick={() => navigate("intake")} type="button">
          <Inbox size={16} /> Intake
        </button>
        <p>PLAN</p>
        <button class:active={activeView === "sprints"} onclick={() => navigate("sprints")} type="button">
          <Gauge size={16} /> Sprints
        </button>
        <button class:active={activeView === "modules"} onclick={() => navigate("modules")} type="button">
          <Blocks size={16} /> Modules
        </button>
        <button class:active={activeView === "pages"} onclick={() => navigate("pages")} type="button">
          <FileText size={16} /> Pages
        </button>
        <p>OBSERVE</p>
        <button class:active={activeView === "analytics"} onclick={() => navigate("analytics")} type="button">
          <BarChart3 size={16} /> Analytics
        </button>
      </nav>

      <div class="sidebar-projects">
        <div><p>PROJECTS</p><button aria-label="Create project" onclick={() => (modal = "project")} type="button"><Plus size={14} /></button></div>
        {#each workspaceProjects as project (project.id)}
          <button class:active={project.id === activeProject?.id} onclick={() => chooseProject(project)} type="button">
            <span>{project.identifier}</span>{project.name}
          </button>
        {:else}
          <p class="empty-nav">No projects yet</p>
        {/each}
      </div>

      <div class="sidebar-footer">
        <button class:active={activeView === "settings"} onclick={() => navigate("settings")} type="button">
          <Settings size={16} /> Settings
        </button>
        <div class="account-row">
          <span>{snapshot.user.display_name.slice(0, 2).toUpperCase()}</span>
          <div><strong>{snapshot.user.display_name}</strong><small>{snapshot.user.email}</small></div>
          <button aria-label="Sign out" disabled={submitting} onclick={logout} type="button"><LogOut size={15} /></button>
        </div>
      </div>
    </aside>

    <main class="workspace-main">
      <header class="topbar">
        <button aria-label="Open navigation" class="icon-button mobile-only" onclick={() => (sidebarOpen = true)} type="button"><Menu size={18} /></button>
        <div class="project-heading">
          <span>{activeProject?.identifier ?? "—"}</span>
          <div><strong>{activeProject?.name ?? "Create a project"}</strong><small>{activeWorkspace?.name}</small></div>
        </div>
        <div class="topbar-actions">
          <div class:online={connected} class="connection-pill">
            {#if connected}<Wifi size={13} /> Live{:else}<WifiOff size={13} /> Reconnecting{/if}
          </div>
          <button class="primary-button compact" disabled={!activeProject} onclick={() => (modal = "ticket")} type="button">
            <Plus size={15} /> New ticket
          </button>
        </div>
      </header>

      {#if errorMessage}<div class="global-alert" role="alert">{errorMessage}<button aria-label="Dismiss error" onclick={() => (errorMessage = "")} type="button"><X size={15} /></button></div>{/if}

      {#if !activeProject && activeView !== "settings"}
        <section class="empty-state">
          <div><Boxes size={32} /></div>
          <p class="eyebrow">YOUR NEXT SYSTEM</p>
          <h1>Create the first project</h1>
          <p>Connect a repository, organize the work, and give your agents a clear place to operate.</p>
          <button class="primary-button" onclick={() => (modal = "project")} type="button"><Plus size={16} /> Create project</button>
        </section>
      {:else if activeView === "tickets"}
        <section class="content-view">
          <div class="view-title">
            <div><p class="eyebrow">DELIVERY QUEUE</p><h1>Tickets</h1><span>{projectTickets.length} open · {urgentTickets} high priority</span></div>
            <div class="view-tools">
              <label class="search-box"><Search size={15} /><span class="sr-only">Search tickets</span><input bind:value={search} placeholder="Search tickets" /></label>
              <div class="segmented" aria-label="Ticket layout">
                <button aria-label="Board view" class:active={ticketLayout === "board"} onclick={() => (ticketLayout = "board")} type="button"><Kanban size={15} /></button>
                <button aria-label="List view" class:active={ticketLayout === "list"} onclick={() => (ticketLayout = "list")} type="button"><List size={15} /></button>
              </div>
            </div>
          </div>
          {#if ticketLayout === "board"}
            <div class="ticket-board">
              <section class="board-column accent-slate">
                <header><span><CircleDot size={14} /> BACKLOG</span><b>{projectTickets.length}</b></header>
                <div class="ticket-stack">
                  {#each projectTickets as ticket (ticket.id)}
                    <article class="ticket-card">
                      <div><span class="ticket-key">{ticketKey(ticket)}</span><span class:urgent={ticket.priority === "urgent"} class="priority">{priorityLabel(ticket.priority)}</span></div>
                      <h2>{ticket.title}</h2>
                      {#if ticket.description}<p>{ticket.description}</p>{/if}
                      <footer><span class="agent-hint"><Bot size={13} /> Ready for agent</span><span>v{ticket.version}</span></footer>
                    </article>
                  {:else}
                    <button class="column-empty" onclick={() => (modal = "ticket")} type="button"><Plus size={18} /><span>Create the first ticket</span></button>
                  {/each}
                </div>
              </section>
              {#each [
                { label: "TODO", className: "accent-cyan" },
                { label: "IN PROGRESS", className: "accent-amber" },
                { label: "DONE", className: "accent-green" },
              ] as column (column.label)}
                <section class={`board-column ${column.className}`}>
                  <header><span><CircleDot size={14} /> {column.label}</span><b>0</b></header>
                  <div class="column-placeholder"><span></span><p>Drop tickets here as work moves.</p></div>
                </section>
              {/each}
            </div>
          {:else}
            <div class="ticket-list">
              <header><span>Ticket</span><span>Priority</span><span>Status</span><span>Agent</span></header>
              {#each projectTickets as ticket (ticket.id)}
                <article><span><b>{ticketKey(ticket)}</b>{ticket.title}</span><span>{priorityLabel(ticket.priority)}</span><span class="status-badge">Backlog</span><span class="agent-hint"><Bot size={13} /> Ready</span></article>
              {:else}
                <div class="list-empty">No tickets match this view.</div>
              {/each}
            </div>
          {/if}
        </section>
      {:else if activeView === "analytics"}
        <section class="content-view">
          <div class="view-title"><div><p class="eyebrow">WORKSPACE SIGNAL</p><h1>Analytics</h1><span>A live read on delivery health.</span></div></div>
          <div class="metric-grid">
            <article><Activity size={18} /><small>ACTIVE TICKETS</small><strong>{projectTickets.length}</strong><p>Across {workspaceProjects.length} projects</p></article>
            <article><Zap size={18} /><small>HIGH PRIORITY</small><strong>{urgentTickets}</strong><p>Needs focused attention</p></article>
            <article><Bot size={18} /><small>AGENT READY</small><strong>{projectTickets.length}</strong><p>Tickets eligible for execution</p></article>
            <article><Check size={18} /><small>CONTROL PLANE</small><strong class="healthy">LIVE</strong><p>Realtime events connected</p></article>
          </div>
          <div class="analytics-panel">
            <div><p class="eyebrow">FLOW OVERVIEW</p><h2>Delivery distribution</h2></div>
            <div class="flow-bars">
              <span style={`--value: ${Math.max(projectTickets.length, 1)}`}><i>Backlog</i><b>{projectTickets.length}</b></span>
              <span style="--value: 0.15"><i>In progress</i><b>0</b></span>
              <span style="--value: 0.08"><i>Done</i><b>0</b></span>
            </div>
          </div>
        </section>
      {:else if activeView === "settings"}
        <section class="content-view narrow">
          <div class="view-title"><div><p class="eyebrow">CONTROL PLANE</p><h1>Settings</h1><span>Manage this client and its connection.</span></div></div>
          <form class="settings-card" onsubmit={saveInstance}>
            <div class="settings-icon"><Cloud size={20} /></div>
            <div><h2>Connected instance</h2><p>Leave blank to use the instance serving this client, or point the web/desktop shell at another Jet Black server.</p></div>
            <label><span>Instance URL</span><input bind:value={instanceDraft} placeholder="https://jet-black.example.com" type="url" /></label>
            <button class="secondary-button" type="submit">Save and reconnect</button>
          </form>
          <div class="settings-card">
            <div class="settings-icon"><Users size={20} /></div>
            <div><h2>Workspace access</h2><p>You are an <strong>{activeWorkspace?.role}</strong> in {activeWorkspace?.name}. Team membership and invitations use control-plane roles.</p></div>
            <span class="role-chip">{activeWorkspace?.role}</span>
          </div>
        </section>
      {:else}
        <section class="content-view">
          <div class="view-title"><div><p class="eyebrow">PROJECT SYSTEM</p><h1>{activeView[0].toUpperCase() + activeView.slice(1)}</h1><span>Organize the context around every delivery.</span></div></div>
          <div class="feature-grid">
            <article class="feature-hero">
              {#if activeView === "intake"}<Inbox size={28} />{:else if activeView === "sprints"}<Gauge size={28} />{:else if activeView === "modules"}<Layers3 size={28} />{:else}<FileText size={28} />{/if}
              <p class="eyebrow">{activeView.toUpperCase()}</p>
              <h2>Your {activeView} live beside the code</h2>
              <p>The Rust control plane is ready for this project domain. Create and organize records here as the workspace evolves.</p>
              <button class="secondary-button" type="button"><Plus size={15} /> Create {activeView === "intake" ? "intake item" : activeView.slice(0, -1)}</button>
            </article>
            <article><Terminal size={18} /><h3>Agent context</h3><p>Every record can become structured, repository-scoped execution context.</p></article>
            <article><Archive size={18} /><h3>Durable history</h3><p>SQLite keeps product state and semantic events together on your instance.</p></article>
          </div>
        </section>
      {/if}
    </main>
  </div>
{/if}

{#if modal}
  <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && resetModal()}>
    <form class="modal-card" onsubmit={submitModal}>
      <header>
        <div><p class="eyebrow">CREATE</p><h2>New {modal}</h2></div>
        <button aria-label="Close dialog" class="icon-button" onclick={resetModal} type="button"><X size={17} /></button>
      </header>
      {#if errorMessage}<div class="alert" role="alert">{errorMessage}</div>{/if}
      {#if modal === "workspace"}
        <label><span>Workspace name</span><input bind:value={workspaceName} placeholder="Platform engineering" required /></label>
        <label><span>URL slug</span><input bind:value={workspaceSlug} pattern="[a-z0-9-]+" placeholder="platform-engineering" required /></label>
      {:else if modal === "project"}
        <div class="field-row">
          <label><span>Identifier</span><input bind:value={projectIdentifier} maxlength="12" pattern="[A-Za-z0-9]+" placeholder="PLAT" required /></label>
          <label><span>Project name</span><input bind:value={projectName} placeholder="Platform" required /></label>
        </div>
        <label><span>Repository identity <small>optional</small></span><input bind:value={repositoryIdentity} placeholder="github:org/repository" /></label>
      {:else}
        <label><span>Title</span><input bind:value={ticketTitle} placeholder="What needs to ship?" required /></label>
        <label><span>Description</span><textarea bind:value={ticketDescription} placeholder="Give the team and agents enough context to act."></textarea></label>
        <label><span>Priority</span><select bind:value={ticketPriority}><option value="none">No priority</option><option value="urgent">Urgent</option><option value="high">High</option><option value="medium">Medium</option><option value="low">Low</option></select></label>
      {/if}
      <footer><button class="ghost-button" onclick={resetModal} type="button">Cancel</button><button class="primary-button compact" disabled={submitting} type="submit">{submitting ? "Creating…" : `Create ${modal}`}</button></footer>
    </form>
  </div>
{/if}
