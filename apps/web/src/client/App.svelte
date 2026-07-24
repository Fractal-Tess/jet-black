<script lang="ts">
import type {
  ApprovedRepositorySummary,
  ProductCommand,
  ProductProject,
  ProductSnapshot,
  ProductTicket,
  ProductTicketPriority,
  ProductWorkspace,
  ProviderKind,
  ReviewReport,
  RunSnapshot,
} from "@workspace/shared/protocol";
import { Button } from "@workspace/ui/components/button";
import { Input } from "@workspace/ui/components/input";
import { Textarea } from "@workspace/ui/components/textarea";
import { cn } from "@workspace/ui/lib/utils";
import {
  Activity,
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
  GitBranch,
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
  Users,
  Wifi,
  WifiOff,
  X,
  Zap,
} from "lucide-svelte";
import { onMount } from "svelte";
import {
  JetBlackClient,
  JetBlackClientError,
  type ManagedUser,
  type RemoteConnection,
  saveConnections,
  savedConnections,
} from "./jet-black-client";

type View =
  | "analytics"
  | "intake"
  | "modules"
  | "pages"
  | "settings"
  | "sprints"
  | "tickets";

const baseStyles =
  "min-h-screen min-w-80 bg-background font-mono text-foreground antialiased selection:bg-primary selection:text-primary-foreground [&_*]:box-border [&_button]:cursor-pointer [&_button:disabled]:cursor-not-allowed [&_button:disabled]:opacity-50 [&_input]:h-10 [&_input]:w-full [&_input]:rounded-md [&_input]:border [&_input]:border-input [&_input]:bg-background [&_input]:px-3 [&_input]:py-2 [&_input]:text-xs [&_input]:outline-none [&_input]:transition-shadow [&_input]:placeholder:text-muted-foreground/70 [&_input:focus]:border-ring [&_input:focus]:ring-2 [&_input:focus]:ring-ring/20 [&_select]:h-10 [&_select]:w-full [&_select]:rounded-md [&_select]:border [&_select]:border-input [&_select]:bg-background [&_select]:px-3 [&_select]:text-xs [&_select]:outline-none [&_select:focus]:border-ring [&_select:focus]:ring-2 [&_select:focus]:ring-ring/20 [&_textarea]:min-h-32 [&_textarea]:w-full [&_textarea]:resize-y [&_textarea]:rounded-md [&_textarea]:border [&_textarea]:border-input [&_textarea]:bg-background [&_textarea]:px-3 [&_textarea]:py-2 [&_textarea]:text-xs [&_textarea]:leading-relaxed [&_textarea]:outline-none [&_textarea:focus]:border-ring [&_textarea:focus]:ring-2 [&_textarea:focus]:ring-ring/20";
const sharedStyles =
  "[&_.sr-only]:sr-only [&_.eyebrow]:mb-2.5 [&_.eyebrow]:flex [&_.eyebrow]:items-center [&_.eyebrow]:gap-2 [&_.eyebrow]:text-[0.68rem] [&_.eyebrow]:font-bold [&_.eyebrow]:tracking-[0.18em] [&_.eyebrow]:text-primary [&_.brand-mark]:grid [&_.brand-mark]:size-8 [&_.brand-mark]:place-items-center [&_.brand-mark]:border [&_.brand-mark]:border-primary [&_.brand-mark]:text-[0.7rem] [&_.brand-mark]:text-primary [&_.brand-lockup]:flex [&_.brand-lockup]:items-center [&_.brand-lockup]:gap-3 [&_.brand-lockup]:text-xs [&_.brand-lockup]:font-extrabold [&_.brand-lockup]:tracking-[0.18em] [&_.primary-button]:inline-flex [&_.primary-button]:min-h-10 [&_.primary-button]:items-center [&_.primary-button]:justify-center [&_.primary-button]:gap-2 [&_.primary-button]:rounded-md [&_.primary-button]:bg-primary [&_.primary-button]:px-4 [&_.primary-button]:text-xs [&_.primary-button]:font-bold [&_.primary-button]:text-primary-foreground [&_.primary-button]:transition-colors [&_.primary-button:hover]:bg-[#7aedf1] [&_.primary-button.compact]:min-h-8 [&_.primary-button.compact]:px-3 [&_.primary-button.compact]:text-[0.7rem] [&_.secondary-button]:inline-flex [&_.secondary-button]:min-h-10 [&_.secondary-button]:items-center [&_.secondary-button]:justify-center [&_.secondary-button]:gap-2 [&_.secondary-button]:rounded-md [&_.secondary-button]:border [&_.secondary-button]:border-input [&_.secondary-button]:bg-secondary [&_.secondary-button]:px-4 [&_.secondary-button]:text-xs [&_.secondary-button]:font-semibold [&_.secondary-button:hover]:border-primary [&_.secondary-button:hover]:text-primary [&_.ghost-button]:inline-flex [&_.ghost-button]:min-h-9 [&_.ghost-button]:items-center [&_.ghost-button]:justify-center [&_.ghost-button]:gap-2 [&_.ghost-button]:rounded-md [&_.ghost-button]:px-3 [&_.ghost-button]:text-xs [&_.ghost-button]:font-semibold [&_.ghost-button]:text-muted-foreground [&_.ghost-button:hover]:bg-muted [&_.ghost-button:hover]:text-foreground [&_.icon-button]:grid [&_.icon-button]:size-9 [&_.icon-button]:place-items-center [&_.icon-button]:rounded-md [&_.icon-button]:border [&_.icon-button]:border-border [&_.icon-button]:bg-card [&_.icon-button]:text-muted-foreground [&_.icon-button:hover]:border-primary [&_.icon-button:hover]:text-primary [&_.alert]:rounded-md [&_.alert]:border [&_.alert]:border-destructive/40 [&_.alert]:bg-destructive/10 [&_.alert]:px-3 [&_.alert]:py-2.5 [&_.alert]:text-xs [&_.alert]:leading-relaxed [&_.alert]:text-destructive [&_.alert.success]:border-success/40 [&_.alert.success]:bg-success/10 [&_.alert.success]:text-success";
const authStyles =
  "[&_.loading-screen]:grid [&_.loading-screen]:min-h-screen [&_.loading-screen]:place-content-center [&_.loading-screen]:justify-items-center [&_.loading-screen]:gap-5 [&_.loading-screen]:text-xs [&_.loading-screen]:uppercase [&_.loading-screen]:tracking-widest [&_.loading-screen]:text-muted-foreground [&_.brand-orbit]:grid [&_.brand-orbit]:size-20 [&_.brand-orbit]:animate-pulse [&_.brand-orbit]:place-items-center [&_.brand-orbit]:rounded-full [&_.brand-orbit]:border [&_.brand-orbit]:border-border [&_.brand-orbit_span]:grid [&_.brand-orbit_span]:size-10 [&_.brand-orbit_span]:place-items-center [&_.brand-orbit_span]:border [&_.brand-orbit_span]:border-primary [&_.brand-orbit_span]:text-primary [&_.auth-screen]:grid [&_.auth-screen]:min-h-screen [&_.auth-screen]:grid-cols-[minmax(0,1.15fr)_minmax(380px,0.85fr)] max-[760px]:[&_.auth-screen]:grid-cols-1 [&_.auth-story]:flex [&_.auth-story]:min-h-screen [&_.auth-story]:flex-col [&_.auth-story]:justify-between [&_.auth-story]:border-r [&_.auth-story]:border-border [&_.auth-story]:p-[clamp(2rem,5vw,5rem)] max-[760px]:[&_.auth-story]:hidden [&_.auth-story_h1]:m-0 [&_.auth-story_h1]:max-w-[820px] [&_.auth-story_h1]:font-sans [&_.auth-story_h1]:text-[clamp(3rem,6vw,6.6rem)] [&_.auth-story_h1]:font-semibold [&_.auth-story_h1]:leading-[0.92] [&_.auth-story_h1]:tracking-[-0.065em] [&_.auth-story_em]:not-italic [&_.auth-story_em]:text-primary [&_.auth-copy]:mt-8 [&_.auth-copy]:max-w-[62ch] [&_.auth-copy]:font-sans [&_.auth-copy]:text-[clamp(1rem,1.4vw,1.2rem)] [&_.auth-copy]:leading-7 [&_.auth-copy]:text-muted-foreground [&_.signal-grid]:grid [&_.signal-grid]:grid-cols-4 [&_.signal-grid]:border [&_.signal-grid]:border-border [&_.signal-grid_span]:border-r [&_.signal-grid_span]:border-border [&_.signal-grid_span]:p-3 [&_.signal-grid_span]:text-[0.62rem] [&_.signal-grid_span]:tracking-[0.16em] [&_.signal-grid_span]:text-muted-foreground [&_.signal-grid_span:last-child]:border-0 [&_.signal-grid_span:last-child]:text-primary [&_.auth-panel]:grid [&_.auth-panel]:place-items-center [&_.auth-panel]:bg-background/80 [&_.auth-panel]:p-8 max-[760px]:[&_.auth-panel]:min-h-screen max-[760px]:[&_.auth-panel]:p-4 [&_.auth-card]:grid [&_.auth-card]:w-[min(100%,420px)] [&_.auth-card]:gap-5 [&_.auth-card]:rounded-lg [&_.auth-card]:border [&_.auth-card]:border-border [&_.auth-card]:bg-card [&_.auth-card]:p-[clamp(1.5rem,4vw,2.5rem)] [&_.auth-card]:shadow-2xl [&_.auth-card_h2]:font-sans [&_.auth-card_h2]:text-2xl [&_.auth-card_p]:mt-2 [&_.auth-card_p]:font-sans [&_.auth-card_p]:text-sm [&_.auth-card_p]:leading-relaxed [&_.auth-card_p]:text-muted-foreground [&_.auth-card_label]:grid [&_.auth-card_label]:gap-2 [&_.auth-card_label]:text-[0.68rem] [&_.auth-card_label]:uppercase [&_.auth-card_label]:tracking-wider [&_.auth-card_label]:text-foreground/75 [&_.auth-switch]:rounded-md [&_.auth-switch]:p-1 [&_.auth-switch]:text-[0.68rem] [&_.auth-switch]:text-muted-foreground [&_.auth-switch:hover]:text-primary [&_.instance-caption]:flex [&_.instance-caption]:items-center [&_.instance-caption]:justify-center [&_.instance-caption]:gap-1.5 [&_.instance-caption]:break-all [&_.instance-caption]:text-center [&_.mobile-brand]:hidden max-[760px]:[&_.mobile-brand]:flex max-[760px]:[&_.mobile-brand]:items-center max-[760px]:[&_.mobile-brand]:gap-3 max-[760px]:[&_.mobile-brand]:text-xs max-[760px]:[&_.mobile-brand]:font-extrabold max-[760px]:[&_.mobile-brand]:tracking-[0.18em]";
const onboardingStyles =
  "[&_.onboarding-screen]:grid [&_.onboarding-screen]:min-h-screen [&_.onboarding-screen]:place-items-center [&_.onboarding-screen]:p-8 [&_.onboarding-card]:grid [&_.onboarding-card]:w-[min(100%,560px)] [&_.onboarding-card]:gap-5 [&_.onboarding-card]:rounded-lg [&_.onboarding-card]:border [&_.onboarding-card]:border-input [&_.onboarding-card]:bg-card [&_.onboarding-card]:p-[clamp(1.5rem,5vw,3rem)] [&_.onboarding-card]:shadow-2xl [&_.onboarding-card_h1]:font-sans [&_.onboarding-card_h1]:text-[clamp(2rem,5vw,3.5rem)] [&_.onboarding-card_h1]:tracking-tight [&_.onboarding-card>p]:font-sans [&_.onboarding-card>p]:leading-relaxed [&_.onboarding-card>p]:text-muted-foreground [&_.onboarding-card>small]:font-sans [&_.onboarding-card>small]:leading-relaxed [&_.onboarding-card>small]:text-muted-foreground [&_.onboarding-card_form]:grid [&_.onboarding-card_form]:gap-4 [&_.onboarding-card_label]:grid [&_.onboarding-card_label]:gap-2 [&_.onboarding-card_label]:text-[0.68rem] [&_.onboarding-card_label]:uppercase [&_.onboarding-card_label]:tracking-wider [&_.onboarding-card_label]:text-foreground/75";
const navigationStyles =
  "[&_.product-shell]:grid [&_.product-shell]:min-h-screen [&_.product-shell]:grid-cols-[250px_minmax(0,1fr)] max-[760px]:[&_.product-shell]:grid-cols-1 [&_.sidebar]:sticky [&_.sidebar]:top-0 [&_.sidebar]:z-30 [&_.sidebar]:flex [&_.sidebar]:h-screen [&_.sidebar]:flex-col [&_.sidebar]:border-r [&_.sidebar]:border-border [&_.sidebar]:bg-[#0b0d11] max-[760px]:[&_.sidebar]:fixed max-[760px]:[&_.sidebar]:left-0 max-[760px]:[&_.sidebar]:w-[min(86vw,290px)] max-[760px]:[&_.sidebar]:-translate-x-[102%] max-[760px]:[&_.sidebar]:transition-transform max-[760px]:[&_.sidebar-visible_.sidebar]:translate-x-0 [&_.sidebar-brand]:flex [&_.sidebar-brand]:h-16 [&_.sidebar-brand]:items-center [&_.sidebar-brand]:gap-3 [&_.sidebar-brand]:border-b [&_.sidebar-brand]:border-border [&_.sidebar-brand]:px-4 [&_.sidebar-brand]:text-[0.7rem] [&_.sidebar-brand]:tracking-[0.16em] [&_.sidebar-brand_.brand-mark]:size-7 [&_.workspace-picker]:relative [&_.workspace-picker]:p-3 [&_.workspace-button]:grid [&_.workspace-button]:w-full [&_.workspace-button]:grid-cols-[2rem_1fr_auto] [&_.workspace-button]:items-center [&_.workspace-button]:gap-2.5 [&_.workspace-button]:rounded-md [&_.workspace-button]:border [&_.workspace-button]:border-border [&_.workspace-button]:bg-card [&_.workspace-button]:p-2 [&_.workspace-button]:text-left [&_.workspace-button:hover]:bg-secondary [&_.workspace-avatar]:grid [&_.workspace-avatar]:size-8 [&_.workspace-avatar]:place-items-center [&_.workspace-avatar]:rounded-sm [&_.workspace-avatar]:bg-primary/10 [&_.workspace-avatar]:text-[0.65rem] [&_.workspace-avatar]:text-primary [&_.workspace-button_small]:block [&_.workspace-button_small]:text-[0.52rem] [&_.workspace-button_small]:tracking-widest [&_.workspace-button_small]:text-muted-foreground [&_.workspace-button_strong]:block [&_.workspace-button_strong]:truncate [&_.workspace-button_strong]:text-[0.7rem] [&_.workspace-menu]:absolute [&_.workspace-menu]:inset-x-3 [&_.workspace-menu]:top-[4.4rem] [&_.workspace-menu]:z-20 [&_.workspace-menu]:rounded-md [&_.workspace-menu]:border [&_.workspace-menu]:border-input [&_.workspace-menu]:bg-popover [&_.workspace-menu]:shadow-2xl [&_.workspace-menu_button]:flex [&_.workspace-menu_button]:w-full [&_.workspace-menu_button]:items-center [&_.workspace-menu_button]:justify-between [&_.workspace-menu_button]:border-b [&_.workspace-menu_button]:border-border [&_.workspace-menu_button]:bg-transparent [&_.workspace-menu_button]:p-3 [&_.workspace-menu_button]:text-left [&_.workspace-menu_button]:text-[0.68rem] [&_.workspace-menu_button:hover]:bg-accent [&_.workspace-menu_button:hover]:text-primary [&_.workspace-menu_small]:uppercase [&_.workspace-menu_small]:text-muted-foreground [&_.workspace-menu_.menu-create]:justify-start [&_.workspace-menu_.menu-create]:gap-2 [&_.workspace-menu_.menu-create]:text-primary [&_.menu-divider]:border-b [&_.menu-divider]:border-border [&_.menu-divider]:px-3 [&_.menu-divider]:py-2 [&_.menu-divider]:text-[0.52rem] [&_.menu-divider]:tracking-[0.16em] [&_.menu-divider]:text-muted-foreground [&_.sidebar_nav]:px-2.5 [&_.sidebar_nav_p]:mb-1.5 [&_.sidebar_nav_p]:mt-4 [&_.sidebar_nav_p]:px-2 [&_.sidebar_nav_p]:text-[0.55rem] [&_.sidebar_nav_p]:tracking-[0.17em] [&_.sidebar_nav_p]:text-muted-foreground [&_.sidebar_nav_button]:flex [&_.sidebar_nav_button]:w-full [&_.sidebar_nav_button]:items-center [&_.sidebar_nav_button]:gap-2.5 [&_.sidebar_nav_button]:border-l-2 [&_.sidebar_nav_button]:border-transparent [&_.sidebar_nav_button]:px-2.5 [&_.sidebar_nav_button]:py-2 [&_.sidebar_nav_button]:text-left [&_.sidebar_nav_button]:text-[0.69rem] [&_.sidebar_nav_button]:text-muted-foreground [&_.sidebar_nav_button:hover]:bg-white/[0.035] [&_.sidebar_nav_button:hover]:text-foreground [&_.sidebar_nav_button.active]:border-primary [&_.sidebar_nav_button.active]:bg-white/[0.035] [&_.sidebar_nav_button.active]:text-primary [&_.sidebar_nav_button_span]:ml-auto [&_.sidebar_nav_button_span]:text-[0.6rem] [&_.sidebar-projects]:min-h-0 [&_.sidebar-projects]:flex-1 [&_.sidebar-projects]:overflow-y-auto [&_.sidebar-projects]:px-2.5 [&_.sidebar-projects>div]:flex [&_.sidebar-projects>div]:items-end [&_.sidebar-projects>div]:justify-between [&_.sidebar-projects_p]:mb-1.5 [&_.sidebar-projects_p]:mt-4 [&_.sidebar-projects_p]:px-2 [&_.sidebar-projects_p]:text-[0.55rem] [&_.sidebar-projects_p]:tracking-[0.17em] [&_.sidebar-projects_p]:text-muted-foreground [&_.sidebar-projects>div_button]:grid [&_.sidebar-projects>div_button]:size-7 [&_.sidebar-projects>div_button]:place-items-center [&_.sidebar-projects>div_button]:text-muted-foreground [&_.sidebar-projects>button]:flex [&_.sidebar-projects>button]:w-full [&_.sidebar-projects>button]:items-center [&_.sidebar-projects>button]:gap-2.5 [&_.sidebar-projects>button]:px-2 [&_.sidebar-projects>button]:py-2 [&_.sidebar-projects>button]:text-left [&_.sidebar-projects>button]:text-[0.66rem] [&_.sidebar-projects>button]:text-muted-foreground [&_.sidebar-projects>button:hover]:bg-white/[0.035] [&_.sidebar-projects>button:hover]:text-foreground [&_.sidebar-projects>button.active]:bg-white/[0.035] [&_.sidebar-projects>button.active]:text-foreground [&_.sidebar-projects>button_span]:grid [&_.sidebar-projects>button_span]:h-6 [&_.sidebar-projects>button_span]:w-7 [&_.sidebar-projects>button_span]:place-items-center [&_.sidebar-projects>button_span]:bg-primary/10 [&_.sidebar-projects>button_span]:text-[0.52rem] [&_.sidebar-projects>button_span]:text-primary [&_.empty-nav]:tracking-normal [&_.sidebar-footer]:border-t [&_.sidebar-footer]:border-border [&_.sidebar-footer]:p-2.5 [&_.sidebar-footer>button]:flex [&_.sidebar-footer>button]:w-full [&_.sidebar-footer>button]:items-center [&_.sidebar-footer>button]:gap-2.5 [&_.sidebar-footer>button]:px-2.5 [&_.sidebar-footer>button]:py-2 [&_.sidebar-footer>button]:text-[0.69rem] [&_.sidebar-footer>button]:text-muted-foreground [&_.sidebar-footer>button:hover]:bg-white/[0.035] [&_.sidebar-footer>button:hover]:text-foreground [&_.sidebar-footer>button.active]:bg-white/[0.035] [&_.sidebar-footer>button.active]:text-foreground [&_.account-row]:grid [&_.account-row]:grid-cols-[1.8rem_1fr_auto] [&_.account-row]:items-center [&_.account-row]:gap-2.5 [&_.account-row]:p-2.5 [&_.account-row>span]:grid [&_.account-row>span]:size-7 [&_.account-row>span]:place-items-center [&_.account-row>span]:rounded-full [&_.account-row>span]:bg-agent/15 [&_.account-row>span]:text-[0.58rem] [&_.account-row>span]:text-agent [&_.account-row_strong]:block [&_.account-row_strong]:max-w-[120px] [&_.account-row_strong]:truncate [&_.account-row_strong]:text-[0.65rem] [&_.account-row_small]:block [&_.account-row_small]:max-w-[120px] [&_.account-row_small]:truncate [&_.account-row_small]:text-[0.54rem] [&_.account-row_small]:text-muted-foreground [&_.account-row_button]:text-muted-foreground [&_.sidebar-scrim]:hidden max-[760px]:[&_.sidebar-visible_.sidebar-scrim]:fixed max-[760px]:[&_.sidebar-visible_.sidebar-scrim]:inset-0 max-[760px]:[&_.sidebar-visible_.sidebar-scrim]:z-20 max-[760px]:[&_.sidebar-visible_.sidebar-scrim]:block max-[760px]:[&_.sidebar-visible_.sidebar-scrim]:bg-black/60 [&_.mobile-only]:hidden max-[760px]:[&_.mobile-only]:grid";
const workspaceStyles =
  "[&_.workspace-main]:min-w-0 [&_.workspace-main]:bg-background [&_.topbar]:sticky [&_.topbar]:top-0 [&_.topbar]:z-10 [&_.topbar]:flex [&_.topbar]:h-16 [&_.topbar]:items-center [&_.topbar]:justify-between [&_.topbar]:gap-4 [&_.topbar]:border-b [&_.topbar]:border-border [&_.topbar]:bg-background/85 [&_.topbar]:px-5 [&_.topbar]:backdrop-blur-md max-[760px]:[&_.topbar]:px-3 [&_.project-heading]:flex [&_.project-heading]:items-center [&_.project-heading]:gap-3 [&_.project-heading>span]:grid [&_.project-heading>span]:h-8 [&_.project-heading>span]:min-w-9 [&_.project-heading>span]:place-items-center [&_.project-heading>span]:bg-primary/10 [&_.project-heading>span]:px-2 [&_.project-heading>span]:text-[0.6rem] [&_.project-heading>span]:text-primary [&_.project-heading_strong]:block [&_.project-heading_strong]:text-xs [&_.project-heading_small]:block [&_.project-heading_small]:text-[0.56rem] [&_.project-heading_small]:text-muted-foreground [&_.topbar-actions]:flex [&_.topbar-actions]:items-center [&_.topbar-actions]:gap-3 [&_.connection-pill]:flex [&_.connection-pill]:items-center [&_.connection-pill]:gap-1.5 [&_.connection-pill]:text-[0.59rem] [&_.connection-pill]:uppercase [&_.connection-pill]:text-warning max-[760px]:[&_.connection-pill]:hidden [&_.connection-pill.online]:text-success [&_.global-alert]:mx-6 [&_.global-alert]:mt-4 [&_.global-alert]:flex [&_.global-alert]:items-center [&_.global-alert]:justify-between [&_.global-alert_button]:text-inherit [&_.content-view]:p-[clamp(1.25rem,3vw,2.5rem)] max-[760px]:[&_.content-view]:p-4 [&_.content-view.narrow]:max-w-[900px] [&_.view-title]:mb-7 [&_.view-title]:flex [&_.view-title]:items-end [&_.view-title]:justify-between [&_.view-title]:gap-4 max-[760px]:[&_.view-title]:flex-col max-[760px]:[&_.view-title]:items-stretch [&_.view-title_h1]:font-sans [&_.view-title_h1]:text-[clamp(1.8rem,3vw,2.65rem)] [&_.view-title_h1]:tracking-tight [&_.view-title>div>span]:mt-2 [&_.view-title>div>span]:block [&_.view-title>div>span]:font-sans [&_.view-title>div>span]:text-sm [&_.view-title>div>span]:text-muted-foreground [&_.view-tools]:flex [&_.view-tools]:gap-2 max-[760px]:[&_.view-tools]:w-full [&_.search-box]:flex [&_.search-box]:items-center [&_.search-box]:gap-2 [&_.search-box]:rounded-md [&_.search-box]:border [&_.search-box]:border-border [&_.search-box]:bg-card [&_.search-box]:pl-3 [&_.search-box]:text-muted-foreground max-[760px]:[&_.search-box]:flex-1 [&_.search-box_input]:w-44 [&_.search-box_input]:border-0 [&_.search-box_input]:bg-transparent [&_.search-box_input]:px-2 [&_.search-box_input]:shadow-none max-[760px]:[&_.search-box_input]:w-full [&_.segmented]:flex [&_.segmented]:rounded-md [&_.segmented]:border [&_.segmented]:border-border [&_.segmented]:bg-card [&_.segmented]:p-1 [&_.segmented_button]:grid [&_.segmented_button]:size-8 [&_.segmented_button]:place-items-center [&_.segmented_button]:rounded-sm [&_.segmented_button]:text-muted-foreground [&_.segmented_button.active]:bg-accent [&_.segmented_button.active]:text-primary [&_.empty-state]:grid [&_.empty-state]:min-h-[calc(100vh-4rem)] [&_.empty-state]:place-content-center [&_.empty-state]:justify-items-center [&_.empty-state]:p-8 [&_.empty-state]:text-center [&_.empty-state>div]:grid [&_.empty-state>div]:size-16 [&_.empty-state>div]:place-items-center [&_.empty-state>div]:border [&_.empty-state>div]:border-border [&_.empty-state>div]:bg-card [&_.empty-state>div]:text-primary [&_.empty-state_h1]:font-sans [&_.empty-state_h1]:text-3xl [&_.empty-state>p:not(.eyebrow)]:max-w-[52ch] [&_.empty-state>p:not(.eyebrow)]:font-sans [&_.empty-state>p:not(.eyebrow)]:leading-relaxed [&_.empty-state>p:not(.eyebrow)]:text-muted-foreground";
const ticketStyles =
  "[&_.ticket-board]:grid [&_.ticket-board]:grid-cols-[repeat(4,minmax(230px,1fr))] [&_.ticket-board]:gap-3 [&_.ticket-board]:overflow-x-auto [&_.ticket-board]:pb-4 max-[1050px]:[&_.ticket-board]:grid-cols-[repeat(4,minmax(260px,1fr))] [&_.board-column]:min-h-[calc(100vh-190px)] [&_.board-column]:border-t-2 [&_.board-column]:border-muted-foreground [&_.board-column]:bg-white/[0.012] [&_.board-column.accent-cyan]:border-primary [&_.board-column.accent-amber]:border-warning [&_.board-column.accent-green]:border-success [&_.board-column>header]:flex [&_.board-column>header]:items-center [&_.board-column>header]:justify-between [&_.board-column>header]:px-3 [&_.board-column>header]:py-3 [&_.board-column>header]:text-[0.59rem] [&_.board-column>header]:tracking-widest [&_.board-column>header]:text-muted-foreground [&_.board-column>header_span]:flex [&_.board-column>header_span]:items-center [&_.board-column>header_span]:gap-1.5 [&_.board-column>header_b]:grid [&_.board-column>header_b]:size-6 [&_.board-column>header_b]:place-items-center [&_.board-column>header_b]:rounded-sm [&_.board-column>header_b]:bg-secondary [&_.board-column>header_b]:text-[0.56rem] [&_.ticket-stack]:grid [&_.ticket-stack]:gap-2 [&_.ticket-stack]:px-2 [&_.ticket-card]:w-full [&_.ticket-card]:rounded-md [&_.ticket-card]:border [&_.ticket-card]:border-border [&_.ticket-card]:border-b-[3px] [&_.ticket-card]:border-b-input [&_.ticket-card]:bg-card [&_.ticket-card]:p-3 [&_.ticket-card]:text-left [&_.ticket-card]:transition-all [&_.ticket-card:hover]:-translate-y-0.5 [&_.ticket-card:hover]:border-b-primary [&_.ticket-card:hover]:bg-secondary [&_.ticket-card:hover]:shadow-xl [&_.ticket-card>div]:flex [&_.ticket-card>div]:items-center [&_.ticket-card>div]:justify-between [&_.ticket-card>div]:gap-2 [&_.ticket-key]:text-[0.58rem] [&_.ticket-key]:text-muted-foreground [&_.priority]:text-[0.53rem] [&_.priority]:text-muted-foreground [&_.priority.urgent]:text-destructive [&_.ticket-card_h2]:mt-2.5 [&_.ticket-card_h2]:font-sans [&_.ticket-card_h2]:text-sm [&_.ticket-card_h2]:leading-snug [&_.ticket-card_p]:mt-2 [&_.ticket-card_p]:line-clamp-2 [&_.ticket-card_p]:font-sans [&_.ticket-card_p]:text-xs [&_.ticket-card_p]:leading-relaxed [&_.ticket-card_p]:text-muted-foreground [&_.ticket-card_footer]:mt-4 [&_.ticket-card_footer]:flex [&_.ticket-card_footer]:items-center [&_.ticket-card_footer]:justify-between [&_.ticket-card_footer]:text-[0.54rem] [&_.ticket-card_footer]:text-muted-foreground [&_.agent-hint]:inline-flex [&_.agent-hint]:items-center [&_.agent-hint]:gap-1 [&_.agent-hint]:text-[0.57rem] [&_.agent-hint]:text-agent [&_.column-empty]:grid [&_.column-empty]:min-h-32 [&_.column-empty]:place-content-center [&_.column-empty]:justify-items-center [&_.column-empty]:gap-2 [&_.column-empty]:rounded-md [&_.column-empty]:border [&_.column-empty]:border-dashed [&_.column-empty]:border-input [&_.column-empty]:text-[0.62rem] [&_.column-empty]:text-muted-foreground [&_.column-empty:hover]:border-primary [&_.column-empty:hover]:text-primary [&_.column-placeholder]:grid [&_.column-placeholder]:min-h-32 [&_.column-placeholder]:place-content-center [&_.column-placeholder]:justify-items-center [&_.column-placeholder]:p-4 [&_.column-placeholder]:text-center [&_.column-placeholder]:text-muted-foreground [&_.column-placeholder_span]:h-0.5 [&_.column-placeholder_span]:w-8 [&_.column-placeholder_span]:bg-border [&_.column-placeholder_p]:max-w-[18ch] [&_.column-placeholder_p]:font-sans [&_.column-placeholder_p]:text-xs [&_.column-placeholder_p]:leading-relaxed [&_.ticket-list]:overflow-hidden [&_.ticket-list]:rounded-md [&_.ticket-list]:border [&_.ticket-list]:border-border [&_.ticket-list]:bg-card max-[760px]:[&_.ticket-list]:overflow-x-auto [&_.ticket-list>header]:grid [&_.ticket-list>header]:grid-cols-[minmax(260px,1fr)_120px_110px_100px] [&_.ticket-list>header]:gap-4 [&_.ticket-list>header]:border-b [&_.ticket-list>header]:border-border [&_.ticket-list>header]:bg-secondary [&_.ticket-list>header]:px-4 [&_.ticket-list>header]:py-3 [&_.ticket-list>header]:text-[0.56rem] [&_.ticket-list>header]:uppercase [&_.ticket-list>header]:tracking-widest [&_.ticket-list>header]:text-muted-foreground max-[760px]:[&_.ticket-list>header]:min-w-[720px] [&_.ticket-list-row]:grid [&_.ticket-list-row]:w-full [&_.ticket-list-row]:grid-cols-[minmax(260px,1fr)_120px_110px_100px] [&_.ticket-list-row]:items-center [&_.ticket-list-row]:gap-4 [&_.ticket-list-row]:border-b [&_.ticket-list-row]:border-border [&_.ticket-list-row]:px-4 [&_.ticket-list-row]:py-3 [&_.ticket-list-row]:text-left [&_.ticket-list-row]:text-[0.66rem] [&_.ticket-list-row]:text-muted-foreground [&_.ticket-list-row:hover]:bg-white/[0.025] [&_.ticket-list-row:hover]:text-foreground max-[760px]:[&_.ticket-list-row]:min-w-[720px] [&_.ticket-list-row>span:first-child]:flex [&_.ticket-list-row>span:first-child]:gap-3 [&_.ticket-list-row>span:first-child]:font-sans [&_.ticket-list-row>span:first-child]:text-xs [&_.ticket-list-row>span:first-child]:text-foreground [&_.status-badge]:text-primary [&_.list-empty]:p-12 [&_.list-empty]:text-center [&_.list-empty]:text-xs [&_.list-empty]:text-muted-foreground";
const cardStyles =
  "[&_.metric-grid]:grid [&_.metric-grid]:grid-cols-4 [&_.metric-grid]:gap-3 max-[1050px]:[&_.metric-grid]:grid-cols-2 max-[760px]:[&_.metric-grid]:grid-cols-1 [&_.metric-grid_article]:rounded-md [&_.metric-grid_article]:border [&_.metric-grid_article]:border-border [&_.metric-grid_article]:border-b-[3px] [&_.metric-grid_article]:border-b-input [&_.metric-grid_article]:bg-card [&_.metric-grid_article]:p-5 [&_.metric-grid_article>svg]:text-primary [&_.metric-grid_small]:mt-6 [&_.metric-grid_small]:block [&_.metric-grid_small]:text-[0.56rem] [&_.metric-grid_small]:tracking-widest [&_.metric-grid_small]:text-muted-foreground [&_.metric-grid_strong]:mt-2 [&_.metric-grid_strong]:block [&_.metric-grid_strong]:font-sans [&_.metric-grid_strong]:text-4xl [&_.metric-grid_strong]:tracking-tight [&_.metric-grid_strong.healthy]:font-mono [&_.metric-grid_strong.healthy]:text-3xl [&_.metric-grid_strong.healthy]:text-success [&_.metric-grid_p]:mt-2 [&_.metric-grid_p]:text-[0.6rem] [&_.metric-grid_p]:text-muted-foreground [&_.analytics-panel]:mt-3 [&_.analytics-panel]:rounded-md [&_.analytics-panel]:border [&_.analytics-panel]:border-border [&_.analytics-panel]:border-b-[3px] [&_.analytics-panel]:border-b-input [&_.analytics-panel]:bg-card [&_.analytics-panel]:p-5 [&_.analytics-panel_h2]:font-sans [&_.analytics-panel_h2]:text-lg [&_.flow-bars]:mt-8 [&_.flow-bars]:grid [&_.flow-bars]:gap-4 [&_.flow-bars_span]:grid [&_.flow-bars_span]:grid-cols-[110px_1fr_30px] [&_.flow-bars_span]:items-center [&_.flow-bars_span]:gap-4 [&_.flow-bars_span]:text-xs [&_.flow-bars_span]:text-muted-foreground [&_.flow-bars_i]:not-italic [&_.flow-bars_b]:text-right [&_.record-grid]:grid [&_.record-grid]:grid-cols-3 [&_.record-grid]:gap-3 max-[1050px]:[&_.record-grid]:grid-cols-2 max-[760px]:[&_.record-grid]:grid-cols-1 [&_.record-grid_article]:grid [&_.record-grid_article]:min-h-48 [&_.record-grid_article]:gap-3 [&_.record-grid_article]:rounded-md [&_.record-grid_article]:border [&_.record-grid_article]:border-border [&_.record-grid_article]:border-b-[3px] [&_.record-grid_article]:border-b-input [&_.record-grid_article]:bg-card [&_.record-grid_article]:p-5 [&_.record-grid_article]:transition-all [&_.record-grid_article:hover]:-translate-y-0.5 [&_.record-grid_article:hover]:border-b-primary [&_.record-grid_article>div]:flex [&_.record-grid_article>div]:items-center [&_.record-grid_article>div]:justify-between [&_.record-grid_article>div]:text-muted-foreground [&_.record-grid_article_svg]:text-primary [&_.record-grid_h2]:font-sans [&_.record-grid_h2]:text-base [&_.record-grid_p]:line-clamp-3 [&_.record-grid_p]:font-sans [&_.record-grid_p]:text-xs [&_.record-grid_p]:leading-relaxed [&_.record-grid_p]:text-muted-foreground [&_.record-grid_article_footer]:mt-auto [&_.record-grid_article_footer]:flex [&_.record-grid_article_footer]:items-center [&_.record-grid_article_footer]:justify-between [&_.record-grid_article_footer]:border-t [&_.record-grid_article_footer]:border-border [&_.record-grid_article_footer]:pt-3 [&_.record-grid_article_footer]:text-[0.56rem] [&_.record-grid_article_footer]:text-muted-foreground [&_.record-empty]:col-span-full [&_.record-empty]:grid [&_.record-empty]:min-h-80 [&_.record-empty]:place-content-center [&_.record-empty]:justify-items-center [&_.record-empty]:rounded-md [&_.record-empty]:border [&_.record-empty]:border-dashed [&_.record-empty]:border-input [&_.record-empty]:bg-card [&_.record-empty]:p-8 [&_.record-empty]:text-center [&_.record-empty_svg]:text-primary [&_.record-empty_h2]:mt-4 [&_.record-empty_h2]:font-sans [&_.record-empty_h2]:text-lg [&_.record-empty_p]:mb-4 [&_.record-empty_p]:max-w-[46ch] [&_.record-empty_p]:font-sans [&_.record-empty_p]:text-xs [&_.record-empty_p]:text-muted-foreground";
const settingsStyles =
  "[&_.settings-card]:mb-3 [&_.settings-card]:grid [&_.settings-card]:grid-cols-[auto_1fr_auto] [&_.settings-card]:items-start [&_.settings-card]:gap-4 [&_.settings-card]:rounded-md [&_.settings-card]:border [&_.settings-card]:border-border [&_.settings-card]:border-b-[3px] [&_.settings-card]:border-b-input [&_.settings-card]:bg-card [&_.settings-card]:p-5 max-[760px]:[&_.settings-card]:grid-cols-[auto_1fr] [&_.settings-card_h2]:font-sans [&_.settings-card_h2]:text-base [&_.settings-card_p]:mt-2 [&_.settings-card_p]:max-w-[70ch] [&_.settings-card_p]:font-sans [&_.settings-card_p]:text-xs [&_.settings-card_p]:leading-relaxed [&_.settings-card_p]:text-muted-foreground [&_.settings-card_label]:col-start-2 [&_.connection-row]:col-start-2 [&_.connection-row]:col-end-[-1] [&_.connection-row]:flex [&_.connection-row]:items-center [&_.connection-row]:justify-between [&_.connection-row]:gap-3 [&_.connection-row]:border-t [&_.connection-row]:border-border [&_.connection-row]:pt-3 [&_.connection-row_span]:grid [&_.connection-row_span]:gap-1 [&_.connection-row_small]:text-xs [&_.connection-row_small]:text-muted-foreground [&_.user-access-row]:col-start-2 [&_.user-access-row]:col-end-[-1] [&_.user-access-row]:flex [&_.user-access-row]:items-center [&_.user-access-row]:justify-between [&_.user-access-row]:gap-3 [&_.user-access-row]:border-t [&_.user-access-row]:border-border [&_.user-access-row]:pt-3 [&_.user-access-row>span]:grid [&_.user-access-row>span]:gap-1 [&_.user-access-row_small]:text-xs [&_.user-access-row_small]:text-muted-foreground [&_.user-access-row_select]:w-auto [&_.user-access-row_select]:min-w-32 [&_.pairing-token]:col-start-2 [&_.pairing-token]:col-end-[-1] [&_.pairing-token]:break-all [&_.pairing-token]:border [&_.pairing-token]:border-border [&_.pairing-token]:bg-background [&_.pairing-token]:p-3 [&_.pairing-token]:text-primary [&_.settings-icon]:grid [&_.settings-icon]:size-10 [&_.settings-icon]:place-items-center [&_.settings-icon]:rounded-md [&_.settings-icon]:bg-primary/10 [&_.settings-icon]:text-primary [&_.role-chip]:rounded-sm [&_.role-chip]:bg-primary/10 [&_.role-chip]:px-2 [&_.role-chip]:py-1.5 [&_.role-chip]:text-[0.58rem] [&_.role-chip]:uppercase [&_.role-chip]:text-primary [&_.field-hint]:flex [&_.field-hint]:items-center [&_.field-hint]:gap-2 [&_.field-hint]:text-[0.68rem] [&_.field-hint]:text-muted-foreground";
const overlayStyles =
  "[&_.drawer-scrim]:fixed [&_.drawer-scrim]:inset-0 [&_.drawer-scrim]:z-40 [&_.drawer-scrim]:bg-black/50 [&_.drawer-scrim]:backdrop-blur-sm [&_.ticket-drawer]:fixed [&_.ticket-drawer]:right-0 [&_.ticket-drawer]:top-0 [&_.ticket-drawer]:z-50 [&_.ticket-drawer]:flex [&_.ticket-drawer]:h-screen [&_.ticket-drawer]:w-[min(520px,100vw)] [&_.ticket-drawer]:flex-col [&_.ticket-drawer]:border-l [&_.ticket-drawer]:border-input [&_.ticket-drawer]:bg-card [&_.ticket-drawer]:shadow-2xl [&_.ticket-drawer>header]:flex [&_.ticket-drawer>header]:items-start [&_.ticket-drawer>header]:justify-between [&_.ticket-drawer>header]:gap-4 [&_.ticket-drawer>header]:border-b [&_.ticket-drawer>header]:border-border [&_.ticket-drawer>header]:p-5 [&_.ticket-drawer_h2]:mt-2 [&_.ticket-drawer_h2]:font-sans [&_.ticket-drawer_h2]:text-xl [&_.drawer-body]:grid [&_.drawer-body]:gap-4 [&_.drawer-body]:overflow-y-auto [&_.drawer-body]:p-5 [&_.ticket-description]:font-sans [&_.ticket-description]:text-sm [&_.ticket-description]:leading-relaxed [&_.ticket-description]:text-muted-foreground [&_.ticket-meta]:grid [&_.ticket-meta]:grid-cols-2 [&_.ticket-meta]:border [&_.ticket-meta]:border-border [&_.ticket-meta_span]:p-3 [&_.ticket-meta_span+span]:border-l [&_.ticket-meta_span+span]:border-border [&_.ticket-meta_small]:mb-1 [&_.ticket-meta_small]:block [&_.ticket-meta_small]:text-[0.52rem] [&_.ticket-meta_small]:tracking-widest [&_.ticket-meta_small]:text-muted-foreground [&_.ticket-state-control]:grid [&_.ticket-state-control]:gap-2 [&_.agent-panel]:grid [&_.agent-panel]:gap-4 [&_.agent-panel]:border [&_.agent-panel]:border-border [&_.agent-panel]:border-t-2 [&_.agent-panel]:border-t-agent [&_.agent-panel]:bg-[#0b0d12] [&_.agent-panel]:p-4 [&_.agent-panel-title]:flex [&_.agent-panel-title]:items-center [&_.agent-panel-title]:justify-between [&_.agent-panel-title>div]:flex [&_.agent-panel-title>div]:items-center [&_.agent-panel-title>div]:gap-2 [&_.agent-panel-title>div]:text-agent [&_.agent-panel-title_small]:block [&_.agent-panel-title_small]:text-[0.5rem] [&_.agent-panel-title_small]:tracking-widest [&_.agent-panel-title_small]:text-muted-foreground [&_.agent-panel-title_strong]:block [&_.agent-panel-title_strong]:text-xs [&_.agent-panel-title_strong]:text-foreground [&_.panel-note]:font-sans [&_.panel-note]:text-xs [&_.panel-note]:leading-relaxed [&_.panel-note]:text-muted-foreground [&_.repo-hint]:font-sans [&_.repo-hint]:text-xs [&_.repo-hint]:leading-relaxed [&_.repo-hint]:text-muted-foreground [&_.repo-hint_code]:break-all [&_.repo-hint_code]:text-primary [&_.run-state]:bg-warning/10 [&_.run-state]:px-2 [&_.run-state]:py-1 [&_.run-state]:text-[0.52rem] [&_.run-state]:uppercase [&_.run-state]:text-warning [&_.state-completed]:bg-success/10 [&_.state-completed]:text-success [&_.state-reviewable]:bg-success/10 [&_.state-reviewable]:text-success [&_.state-failed]:bg-destructive/10 [&_.state-failed]:text-destructive [&_.state-interrupted]:bg-destructive/10 [&_.state-interrupted]:text-destructive [&_.run-timeline]:grid [&_.run-timeline]:gap-2.5 [&_.run-timeline]:border [&_.run-timeline]:border-border [&_.run-timeline]:p-3 [&_.run-timeline_span]:grid [&_.run-timeline_span]:grid-cols-[1.6rem_1fr_auto] [&_.run-timeline_span]:items-center [&_.run-timeline_span]:gap-2 [&_.run-timeline_span]:text-muted-foreground [&_.run-timeline_span.complete]:text-success [&_.run-timeline_i]:grid [&_.run-timeline_i]:size-6 [&_.run-timeline_i]:place-items-center [&_.run-timeline_i]:rounded-full [&_.run-timeline_i]:border [&_.run-timeline_i]:border-current [&_.run-timeline_b]:text-xs [&_.run-timeline_b]:text-foreground [&_.run-timeline_small]:text-[0.52rem] [&_.approval-card]:grid [&_.approval-card]:gap-3 [&_.approval-card]:border [&_.approval-card]:border-warning/40 [&_.approval-card]:bg-warning/5 [&_.approval-card]:p-3 [&_.approval-card_code]:truncate [&_.approval-card_code]:text-xs [&_.approval-card_code]:text-warning [&_.approval-card>div]:flex [&_.approval-card>div]:justify-end [&_.approval-card>div]:gap-2 [&_.danger-button]:min-h-10 [&_.danger-button]:rounded-md [&_.danger-button]:border [&_.danger-button]:border-destructive/40 [&_.danger-button]:bg-destructive/10 [&_.danger-button]:text-xs [&_.danger-button]:text-destructive [&_.danger-button:hover]:bg-destructive/20 [&_.review-summary]:grid [&_.review-summary]:gap-3 [&_.review-summary]:border [&_.review-summary]:border-border [&_.review-summary]:p-3 [&_.review-summary>div]:grid [&_.review-summary>div]:gap-1.5 [&_.review-summary_span]:flex [&_.review-summary_span]:justify-between [&_.review-summary_span]:text-[0.6rem] [&_.review-summary_span]:capitalize [&_.review-summary_span]:text-destructive [&_.review-summary_span.passed]:text-success [&_.modal-backdrop]:fixed [&_.modal-backdrop]:inset-0 [&_.modal-backdrop]:z-[100] [&_.modal-backdrop]:grid [&_.modal-backdrop]:place-items-center [&_.modal-backdrop]:bg-black/70 [&_.modal-backdrop]:p-4 [&_.modal-backdrop]:backdrop-blur-md [&_.modal-card]:grid [&_.modal-card]:w-[min(100%,560px)] [&_.modal-card]:gap-4 [&_.modal-card]:rounded-lg [&_.modal-card]:border [&_.modal-card]:border-input [&_.modal-card]:bg-popover [&_.modal-card]:p-5 [&_.modal-card]:shadow-2xl [&_.modal-card_header]:flex [&_.modal-card_header]:items-center [&_.modal-card_header]:justify-between [&_.modal-card_header]:gap-4 [&_.modal-card_footer]:flex [&_.modal-card_footer]:items-center [&_.modal-card_footer]:justify-end [&_.modal-card_footer]:gap-2 [&_.modal-card_footer]:pt-2 [&_.modal-card_h2]:font-sans [&_.modal-card_h2]:text-xl [&_.modal-card_label]:grid [&_.modal-card_label]:gap-2 [&_.modal-card_label]:text-[0.68rem] [&_.modal-card_label]:uppercase [&_.modal-card_label]:tracking-wider [&_.modal-card_label]:text-foreground/75 [&_.field-row]:grid [&_.field-row]:grid-cols-[0.7fr_1.3fr] [&_.field-row]:gap-3 max-[520px]:[&_.field-row]:grid-cols-1";

let client = $state(new JetBlackClient());
let bootstrapProfile = $state("");
let snapshot = $state<ProductSnapshot | null>(null);
let localSnapshot = $state<ProductSnapshot | null>(null);
let remoteConnections = $state<RemoteConnection[]>(savedConnections());
let activeConnectionId = $state("local");
let activeWorkspaceId = $state<string | null>(null);
let activeProjectId = $state<string | null>(null);
let activeView = $state<View>("tickets");
let loading = $state(true);
let submitting = $state(false);
let connected = $state(false);
let sidebarOpen = $state(false);
let createMenuOpen = $state(false);
let modal = $state<
  | "intake"
  | "module"
  | "page"
  | "project"
  | "sprint"
  | "ticket"
  | "workspace"
  | "connection"
  | null
>(null);
let errorMessage = $state("");
let loginEmail = $state("dev@jet-black.local");
let loginPassword = $state("");
let authMode = $state<"login" | "signup">("login");
let signupName = $state("");
let pendingApproval = $state(false);
let search = $state("");
let ticketLayout = $state<"board" | "list">("board");
let workspaceName = $state("");
let workspaceSlug = $state("");
let projectName = $state("");
let projectIdentifier = $state("");
let repositoryIdentity = $state("");
let repositoryKind = $state<"local" | "none" | "remote">("none");
let repositoryLocation = $state("");
let connectionName = $state("");
let connectionUrl = $state("");
let connectionToken = $state("");
let pairingToken = $state("");
let pairingExpiresAt = $state(0);
let managedUsers = $state<ManagedUser[]>([]);
let membershipWorkspaceId = $state("");
let membershipRole = $state<"admin" | "guest" | "member">("member");
let ticketTitle = $state("");
let ticketDescription = $state("");
let ticketPriority = $state<ProductTicketPriority>("none");
let recordName = $state("");
let recordDescription = $state("");
let recordContent = $state("");
let recordEmail = $state("");
let recordStartDate = $state("");
let recordEndDate = $state("");
let recordTargetDate = $state("");
let realtimeCleanup: (() => void) | undefined;
let runPollTimer: ReturnType<typeof setTimeout> | undefined;
let selectedTicketId = $state<string | null>(null);
let executionLoading = $state(false);
let executionBootstrap = $state<{
  default_provider: { kind: string; model: string | null };
  provider_availability: string[];
} | null>(null);
let approvedRepositories = $state<ApprovedRepositorySummary[]>([]);
let selectedRepositoryId = $state("");
let selectedProvider = $state<ProviderKind>("mock");
let runSnapshot = $state<RunSnapshot | null>(null);
let reviewReport = $state<ReviewReport | null>(null);

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
const projectSprints = $derived(
  snapshot?.sprints.filter(
    (sprint) => sprint.project_id === activeProject?.id
  ) ?? []
);
const projectModules = $derived(
  snapshot?.modules.filter(
    (module) => module.project_id === activeProject?.id
  ) ?? []
);
const projectPages = $derived(
  snapshot?.pages.filter((page) => page.project_id === activeProject?.id) ?? []
);
const projectIntake = $derived(
  snapshot?.intake.filter((item) => item.project_id === activeProject?.id) ?? []
);
const urgentTickets = $derived(
  projectTickets.filter(
    (ticket) => ticket.priority === "urgent" || ticket.priority === "high"
  ).length
);
const selectedTicket = $derived(
  projectTickets.find((ticket) => ticket.id === selectedTicketId) ?? null
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
    if (runPollTimer) {
      clearTimeout(runPollTimer);
    }
    window.removeEventListener("popstate", popstate);
  };
});

async function initialize(): Promise<void> {
  loading = true;
  errorMessage = "";
  try {
    const bootstrap = await client.bootstrap();
    bootstrapProfile = bootstrap.profile;
    if (bootstrap.protocol_version !== "1.0") {
      throw new Error(
        `Instance protocol ${bootstrap.protocol_version} is not supported.`
      );
    }
    try {
      await client.currentSession();
    } catch (error) {
      if (
        error instanceof JetBlackClientError &&
        error.status === 401 &&
        bootstrap.authentication === "local_onboarding"
      ) {
        await client.localLogin();
      } else {
        throw error;
      }
    }
    await refreshSnapshot();
  } catch (error) {
    if (!(error instanceof JetBlackClientError && error.status === 401)) {
      errorMessage = readableError(error);
    }
  } finally {
    loading = false;
  }
}

async function signup(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  submitting = true;
  errorMessage = "";
  try {
    const result = await client.signup({
      display_name: signupName,
      email: loginEmail,
      password: loginPassword,
    });
    if (result.status === "pending_approval") {
      pendingApproval = true;
      return;
    }
    await refreshSnapshot();
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    submitting = false;
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
  if (activeConnectionId === "local") {
    localSnapshot = next;
  }
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

async function switchConnection(connectionId: string): Promise<void> {
  realtimeCleanup?.();
  errorMessage = "";
  activeConnectionId = connectionId;
  activeWorkspaceId = null;
  activeProjectId = null;
  if (connectionId === "local") {
    client = new JetBlackClient();
    snapshot = localSnapshot;
    await refreshSnapshot();
    return;
  }
  const connection = remoteConnections.find((item) => item.id === connectionId);
  if (!connection) {
    return;
  }
  client = new JetBlackClient(connection.baseUrl, {
    csrfToken: connection.csrfToken,
    sessionToken: connection.sessionToken,
  });
  await refreshSnapshot();
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
  if (view === "settings") {
    loadManagedUsers();
  }
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
    if (modal === "connection") {
      await connectRemoteWorkspace();
      return;
    }
    const command = buildModalCommand();
    if (!command) {
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

async function connectRemoteWorkspace(): Promise<void> {
  const connection = await JetBlackClient.connectRemote({
    baseUrl: connectionUrl,
    deviceName: navigator.userAgent.includes("Tauri")
      ? "Jet Black desktop"
      : "Jet Black web client",
    name: connectionName,
    token: connectionToken,
  });
  remoteConnections = [...remoteConnections, connection];
  saveConnections(remoteConnections);
  resetModal();
  await switchConnection(connection.id);
}

function buildModalCommand(): ProductCommand | null {
  switch (modal) {
    case "workspace":
      return {
        type: "create_workspace",
        data: { name: workspaceName, slug: workspaceSlug },
      };
    case "project":
      return activeWorkspace
        ? {
            type: "create_project",
            data: {
              description: "",
              identifier: projectIdentifier,
              name: projectName,
              repository_identity: repositoryIdentity || null,
              repository_kind:
                repositoryKind === "none" ? null : repositoryKind,
              repository_location: repositoryLocation || null,
              workspace_id: activeWorkspace.id,
            },
          }
        : null;
    case "ticket":
      return activeProject
        ? {
            type: "create_ticket",
            data: {
              description: ticketDescription,
              idempotency_key: crypto.randomUUID(),
              priority: ticketPriority,
              project_id: activeProject.id,
              title: ticketTitle,
            },
          }
        : null;
    case "sprint":
      return activeProject
        ? {
            type: "create_sprint",
            data: {
              description: recordDescription,
              ends_at_ms: dateToTimestamp(recordEndDate),
              name: recordName,
              project_id: activeProject.id,
              starts_at_ms: dateToTimestamp(recordStartDate),
            },
          }
        : null;
    case "module":
      return activeProject
        ? {
            type: "create_module",
            data: {
              description: recordDescription,
              name: recordName,
              project_id: activeProject.id,
              target_at_ms: dateToTimestamp(recordTargetDate),
            },
          }
        : null;
    case "page":
      return activeProject
        ? {
            type: "create_page",
            data: {
              content: recordContent,
              project_id: activeProject.id,
              title: recordName,
            },
          }
        : null;
    case "intake":
      return activeProject
        ? {
            type: "create_intake_item",
            data: {
              description: recordDescription,
              project_id: activeProject.id,
              submitter_email: recordEmail || null,
              title: recordName,
            },
          }
        : null;
    default:
      return null;
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
  repositoryKind = "none";
  repositoryLocation = "";
  connectionName = "";
  connectionUrl = "";
  connectionToken = "";
  ticketTitle = "";
  ticketDescription = "";
  ticketPriority = "none";
  recordName = "";
  recordDescription = "";
  recordContent = "";
  recordEmail = "";
  recordStartDate = "";
  recordEndDate = "";
  recordTargetDate = "";
}

function dateToTimestamp(value: string): number | null {
  return value ? new Date(`${value}T00:00:00Z`).getTime() : null;
}

function openFeatureModal(): void {
  if (activeView === "sprints") {
    modal = "sprint";
  } else if (activeView === "modules") {
    modal = "module";
  } else if (activeView === "pages") {
    modal = "page";
  } else if (activeView === "intake") {
    modal = "intake";
  }
}

async function createLocalWorkspace(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  submitting = true;
  errorMessage = "";
  try {
    await client.command({
      type: "create_workspace",
      data: { name: workspaceName, slug: workspaceSlug },
    });
    await refreshSnapshot();
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    submitting = false;
  }
}

async function generatePairingToken(): Promise<void> {
  submitting = true;
  try {
    const result = await client.issueDesktopToken("Jet Black desktop");
    pairingToken = result.token;
    pairingExpiresAt = result.expires_at_ms;
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    submitting = false;
  }
}

async function loadManagedUsers(): Promise<void> {
  try {
    managedUsers = await client.managedUsers();
  } catch (error) {
    if (!(error instanceof JetBlackClientError && error.status === 403)) {
      errorMessage = readableError(error);
    }
  }
}

async function approveManagedUser(userId: string): Promise<void> {
  await client.approveUser(userId);
  await loadManagedUsers();
}

async function assignManagedUser(userId: string): Promise<void> {
  if (!membershipWorkspaceId) {
    return;
  }
  await client.assignWorkspaceMember(
    membershipWorkspaceId,
    userId,
    membershipRole
  );
  await refreshSnapshot();
}

function disconnectRemote(connectionId: string): void {
  remoteConnections = remoteConnections.filter(
    (connection) => connection.id !== connectionId
  );
  saveConnections(remoteConnections);
  if (activeConnectionId === connectionId) {
    switchConnection("local").catch((error) => {
      errorMessage = readableError(error);
    });
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

function ticketStateGroup(ticket: ProductTicket): string {
  return (
    snapshot?.workflow_states.find((state) => state.id === ticket.state_id)
      ?.state_group ?? "backlog"
  );
}

function ticketsInState(stateGroup: string): ProductTicket[] {
  return projectTickets.filter(
    (ticket) => ticketStateGroup(ticket) === stateGroup
  );
}

async function moveTicket(stateGroup: string): Promise<void> {
  if (!selectedTicket) {
    return;
  }
  submitting = true;
  try {
    await client.command({
      type: "move_ticket",
      data: {
        expected_version: selectedTicket.version,
        state_group: stateGroup,
        ticket_id: selectedTicket.id,
      },
    });
    await refreshSnapshot();
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    submitting = false;
  }
}

async function openTicket(ticket: ProductTicket): Promise<void> {
  selectedTicketId = ticket.id;
  runSnapshot = null;
  reviewReport = null;
  executionLoading = true;
  errorMessage = "";
  try {
    const [bootstrap, repositories] = await Promise.all([
      client.executionBootstrap(),
      client.executionCommand({ type: "list_approved_repositories" }),
    ]);
    executionBootstrap = bootstrap;
    selectedProvider = bootstrap.default_provider.kind as ProviderKind;
    if (repositories.type === "approved_repositories") {
      approvedRepositories = repositories.data.repositories;
      selectedRepositoryId = approvedRepositories[0]?.id ?? "";
    }
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    executionLoading = false;
  }
}

async function startTicketRun(): Promise<void> {
  if (!(selectedTicket && activeProject && activeWorkspace)) {
    return;
  }
  if (!selectedRepositoryId) {
    errorMessage =
      "Approve a repository in this instance before starting a run.";
    return;
  }
  executionLoading = true;
  errorMessage = "";
  try {
    const repository = await client.executionCommand({
      type: "register_repository",
      data: { approved_repository_id: selectedRepositoryId },
    });
    if (repository.type !== "repository_registered") {
      throw new Error(
        "Repository registration returned an unexpected response."
      );
    }
    const changeset = await client.executionCommand({
      type: "create_changeset",
      data: {
        base_sha: repository.data.base_sha,
        repository_id: repository.data.id,
        ticket: {
          control_plane_id: null,
          identifier: ticketKey(selectedTicket),
          project_id: activeProject.id,
          ticket_id: selectedTicket.id,
          title: selectedTicket.title,
          workspace_id: activeWorkspace.id,
        },
      },
    });
    if (changeset.type !== "changeset_created") {
      throw new Error("Changeset creation returned an unexpected response.");
    }
    const started = await client.executionCommand({
      type: "start_run",
      data: {
        changeset_id: changeset.data.id,
        provider_selection: { kind: selectedProvider, model: null },
      },
    });
    if (started.type !== "run_started") {
      throw new Error("Run start returned an unexpected response.");
    }
    await pollRun(started.data.run_id);
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    executionLoading = false;
  }
}

async function pollRun(runId: string): Promise<void> {
  const response = await client.executionCommand({
    type: "get_snapshot",
    data: { run_id: runId },
  });
  if (response.type !== "snapshot") {
    throw new Error("Run snapshot returned an unexpected response.");
  }
  runSnapshot = response.data;
  if (["queued", "starting", "running"].includes(response.data.run.state)) {
    runPollTimer = setTimeout(() => {
      pollRun(runId).catch((error) => {
        errorMessage = readableError(error);
      });
    }, 500);
  }
}

async function respondToApproval(approved: boolean): Promise<void> {
  const pending = runSnapshot?.pending_approval;
  if (!(runSnapshot && pending)) {
    return;
  }
  executionLoading = true;
  try {
    await client.executionCommand({
      type: "respond_to_approval",
      data: {
        approved,
        run_id: runSnapshot.run.id,
        scope: pending.scope,
      },
    });
    await pollRun(runSnapshot.run.id);
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    executionLoading = false;
  }
}

async function interruptRun(): Promise<void> {
  if (!runSnapshot) {
    return;
  }
  executionLoading = true;
  try {
    await client.executionCommand({
      type: "interrupt_run",
      data: { run_id: runSnapshot.run.id },
    });
    await pollRun(runSnapshot.run.id);
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    executionLoading = false;
  }
}

async function reviewChanges(): Promise<void> {
  if (!runSnapshot) {
    return;
  }
  executionLoading = true;
  try {
    const response = await client.executionCommand({
      type: "review_changeset",
      data: {
        changeset_id: runSnapshot.changeset.id,
        checks: ["format", "typecheck", "test", "secret_scan"],
        expected_head_sha: runSnapshot.changeset.head_sha,
        expected_version: runSnapshot.changeset.version,
      },
    });
    if (response.type === "review_completed") {
      reviewReport = response.data;
    }
  } catch (error) {
    errorMessage = readableError(error);
  } finally {
    executionLoading = false;
  }
}
</script>

<svelte:head>
  <title>{activeProject ? `${activeProject.name} · Jet Black` : "Jet Black"}</title>
</svelte:head>

<div
  class={cn(
    baseStyles,
    sharedStyles,
    authStyles,
    onboardingStyles,
    navigationStyles,
    workspaceStyles,
    ticketStyles,
    cardStyles,
    settingsStyles,
    overlayStyles
  )}
>
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
      <form class="auth-card" onsubmit={authMode === "login" ? login : signup}>
        <div class="mobile-brand"><span class="brand-mark">JB</span> JET BLACK</div>
        <div>
          <p class="eyebrow">{authMode === "login" ? "WELCOME BACK" : "REQUEST ACCESS"}</p>
          <h2>{authMode === "login" ? "Sign in to your instance" : "Create your account"}</h2>
          <p>{authMode === "login" ? "Use the account managed by this Jet Black control plane." : "An administrator will approve your account and assign workspaces."}</p>
        </div>
        {#if pendingApproval}
          <div class="alert success" role="status">Your account is waiting for administrator approval.</div>
        {/if}
        {#if errorMessage}<div class="alert" role="alert">{errorMessage}</div>{/if}
        {#if authMode === "signup"}
          <label>
            <span>Name</span>
            <Input autocomplete="name" bind:value={signupName} name="name" placeholder="Your name" required />
          </label>
        {/if}
        <label>
          <span>Email</span>
          <Input
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
          <Input
            autocomplete="current-password"
            bind:value={loginPassword}
            minlength={12}
            name="password"
            placeholder="••••••••••••"
            required
            type="password"
          />
        </label>
        <Button disabled={submitting} type="submit">
          {submitting ? "Working…" : authMode === "login" ? "Enter workspace" : "Request access"}
          <Zap size={16} />
        </Button>
        <Button class="auth-switch" onclick={() => (authMode = authMode === "login" ? "signup" : "login")} type="button" variant="ghost">
          {authMode === "login" ? "Need an account? Sign up" : "Already approved? Sign in"}
        </Button>
        <p class="instance-caption">
          <Cloud size={13} />
          {client.baseUrl || "This device · local instance"}
        </p>
      </form>
    </section>
  </main>
{:else if snapshot.workspaces.length === 0}
  <main class="onboarding-screen">
    <section class="onboarding-card">
      <div class="brand-lockup"><span class="brand-mark">JB</span> JET BLACK</div>
      <p class="eyebrow"><Sparkles size={14} /> LOCAL-FIRST SETUP</p>
      <h1>Create your workspace</h1>
      <p>A workspace is your collection of projects. Nothing to register, no account to configure—this one lives on your device.</p>
      {#if errorMessage}<div class="alert" role="alert">{errorMessage}</div>{/if}
      <form onsubmit={createLocalWorkspace}>
        <label><span>Workspace name</span><Input bind:value={workspaceName} oninput={() => (workspaceSlug = workspaceName.toLowerCase().trim().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, ""))} placeholder="My workspace" required /></label>
        <label><span>Workspace slug</span><Input bind:value={workspaceSlug} pattern="[a-z0-9-]+" placeholder="my-workspace" required /></label>
        <Button disabled={submitting} type="submit">{submitting ? "Creating…" : "Create local workspace"} <Zap size={16} /></Button>
      </form>
      <Button onclick={() => (modal = "connection")} type="button" variant="outline"><Cloud size={15} /> Connect to a remote workspace</Button>
      <small>Local projects can point at repositories already on disk. Jet Black will read their Git origin automatically.</small>
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
            <div class="menu-divider">CONTROL PLANES</div>
            {#if activeConnectionId !== "local"}
              <button onclick={() => switchConnection("local")} type="button"><span>This device</span><small>local</small></button>
            {/if}
            {#each remoteConnections as connection (connection.id)}
              <button onclick={() => switchConnection(connection.id)} type="button">
                <span>{connection.name}</span><small>{new URL(connection.baseUrl).host}</small>
              </button>
            {/each}
            <button class="menu-create" onclick={() => (modal = "workspace")} type="button">
              <Plus size={14} /> New workspace
            </button>
            <button class="menu-create" onclick={() => (modal = "connection")} type="button">
              <Cloud size={14} /> Connect remote
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
          <div><strong>{snapshot.user.display_name}</strong><small>{snapshot.user.email.endsWith(".invalid") ? "This device" : snapshot.user.email}</small></div>
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
                <header><span><CircleDot size={14} /> BACKLOG</span><b>{ticketsInState("backlog").length}</b></header>
                <div class="ticket-stack">
                  {#each ticketsInState("backlog") as ticket (ticket.id)}
                    <button class="ticket-card" onclick={() => openTicket(ticket)} type="button">
                      <div><span class="ticket-key">{ticketKey(ticket)}</span><span class:urgent={ticket.priority === "urgent"} class="priority">{priorityLabel(ticket.priority)}</span></div>
                      <h2>{ticket.title}</h2>
                      {#if ticket.description}<p>{ticket.description}</p>{/if}
                      <footer><span class="agent-hint"><Bot size={13} /> Ready for agent</span><span>v{ticket.version}</span></footer>
                    </button>
                  {:else}
                    <button class="column-empty" onclick={() => (modal = "ticket")} type="button"><Plus size={18} /><span>Create the first ticket</span></button>
                  {/each}
                </div>
              </section>
              {#each [
                { label: "TODO", className: "accent-cyan", stateGroup: "unstarted" },
                { label: "IN PROGRESS", className: "accent-amber", stateGroup: "started" },
                { label: "DONE", className: "accent-green", stateGroup: "completed" },
              ] as column (column.stateGroup)}
                <section class={`board-column ${column.className}`}>
                  <header><span><CircleDot size={14} /> {column.label}</span><b>{ticketsInState(column.stateGroup).length}</b></header>
                  <div class="ticket-stack">
                    {#each ticketsInState(column.stateGroup) as ticket (ticket.id)}
                      <button class="ticket-card" onclick={() => openTicket(ticket)} type="button">
                        <div><span class="ticket-key">{ticketKey(ticket)}</span><span class:urgent={ticket.priority === "urgent"} class="priority">{priorityLabel(ticket.priority)}</span></div>
                        <h2>{ticket.title}</h2>
                        {#if ticket.description}<p>{ticket.description}</p>{/if}
                        <footer><span class="agent-hint"><Bot size={13} /> Ready for agent</span><span>v{ticket.version}</span></footer>
                      </button>
                    {:else}
                      <div class="column-placeholder"><span></span><p>Move tickets here as work progresses.</p></div>
                    {/each}
                  </div>
                </section>
              {/each}
            </div>
          {:else}
            <div class="ticket-list">
              <header><span>Ticket</span><span>Priority</span><span>Status</span><span>Agent</span></header>
              {#each projectTickets as ticket (ticket.id)}
                <button class="ticket-list-row" onclick={() => openTicket(ticket)} type="button"><span><b>{ticketKey(ticket)}</b>{ticket.title}</span><span>{priorityLabel(ticket.priority)}</span><span class="status-badge">{ticketStateGroup(ticket).replaceAll("_", " ")}</span><span class="agent-hint"><Bot size={13} /> Ready</span></button>
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
              <span><i>Backlog</i><progress class="h-2 w-full overflow-hidden rounded-full bg-secondary accent-primary [&::-moz-progress-bar]:bg-primary [&::-webkit-progress-bar]:bg-secondary [&::-webkit-progress-value]:bg-primary" max={Math.max(projectTickets.length, 1)} value={projectTickets.length}></progress><b>{projectTickets.length}</b></span>
              <span><i>In progress</i><progress class="h-2 w-full overflow-hidden rounded-full bg-secondary accent-warning [&::-moz-progress-bar]:bg-warning [&::-webkit-progress-bar]:bg-secondary [&::-webkit-progress-value]:bg-warning" max="1" value="0"></progress><b>0</b></span>
              <span><i>Done</i><progress class="h-2 w-full overflow-hidden rounded-full bg-secondary accent-success [&::-moz-progress-bar]:bg-success [&::-webkit-progress-bar]:bg-secondary [&::-webkit-progress-value]:bg-success" max="1" value="0"></progress><b>0</b></span>
            </div>
          </div>
        </section>
      {:else if activeView === "settings"}
        <section class="content-view narrow">
          <div class="view-title"><div><p class="eyebrow">CONTROL PLANE</p><h1>Settings</h1><span>Manage this client and its connection.</span></div></div>
          <div class="settings-card">
            <div class="settings-icon"><Cloud size={20} /></div>
            <div><h2>Workspace connections</h2><p>Keep local work here, and attach shared control planes with a one-time access token.</p></div>
            <button class="secondary-button" onclick={() => (modal = "connection")} type="button"><Cloud size={14} /> Connect control plane</button>
            {#each remoteConnections as connection (connection.id)}
              <div class="connection-row"><span><strong>{connection.name}</strong><small>{connection.baseUrl}</small></span><button class="ghost-button" onclick={() => disconnectRemote(connection.id)} type="button">Disconnect</button></div>
            {/each}
          </div>
          {#if bootstrapProfile !== "desktop"}
            <div class="settings-card">
              <div class="settings-icon"><Zap size={20} /></div>
              <div><h2>Desktop access token</h2><p>Generate a single-use token, then paste it into Jet Black Desktop. It expires in ten minutes and cannot be reused.</p></div>
              <button class="secondary-button" disabled={submitting} onclick={generatePairingToken} type="button">Generate token</button>
              {#if pairingToken}<code class="pairing-token">{pairingToken}</code><small>Expires {new Date(pairingExpiresAt).toLocaleTimeString()}</small>{/if}
            </div>
          {/if}
          <div class="settings-card">
            <div class="settings-icon"><Users size={20} /></div>
            <div><h2>Workspace access</h2><p>You are an <strong>{activeWorkspace?.role}</strong> in {activeWorkspace?.name}. Team membership and invitations use control-plane roles.</p></div>
            <span class="role-chip">{activeWorkspace?.role}</span>
          </div>
          {#if managedUsers.length > 0}
            <div class="settings-card admin-card">
              <div class="settings-icon"><Users size={20} /></div>
              <div><h2>People and access</h2><p>Approve accounts, then assign access at the workspace level.</p></div>
              {#each managedUsers as managed (managed.user.id)}
                <div class="user-access-row">
                  <span><strong>{managed.user.display_name}</strong><small>{managed.user.email}</small></span>
                  {#if managed.pending}
                    <button class="secondary-button" onclick={() => approveManagedUser(managed.user.id)} type="button">Approve</button>
                  {:else if !managed.instance_admin}
                    <select aria-label={`Workspace for ${managed.user.display_name}`} bind:value={membershipWorkspaceId}><option value="">Choose workspace</option>{#each snapshot.workspaces as workspace (workspace.id)}<option value={workspace.id}>{workspace.name}</option>{/each}</select>
                    <select aria-label={`Role for ${managed.user.display_name}`} bind:value={membershipRole}><option value="member">Member</option><option value="admin">Admin</option><option value="guest">Guest</option></select>
                    <button class="secondary-button" onclick={() => assignManagedUser(managed.user.id)} type="button">Assign</button>
                  {:else}
                    <span class="role-chip">instance admin</span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {:else}
        <section class="content-view">
          <div class="view-title">
            <div><p class="eyebrow">PROJECT SYSTEM</p><h1>{activeView[0].toUpperCase() + activeView.slice(1)}</h1><span>Organize the context around every delivery.</span></div>
            <button class="primary-button compact" onclick={openFeatureModal} type="button"><Plus size={15} /> New {activeView === "intake" ? "intake item" : activeView.slice(0, -1)}</button>
          </div>
          <div class="record-grid">
            {#if activeView === "sprints"}
              {#each projectSprints as sprint (sprint.id)}
                <article><div><Gauge size={17} /><span class="status-badge">{sprint.status}</span></div><h2>{sprint.name}</h2><p>{sprint.description || "No sprint brief yet."}</p><footer><span>{sprint.starts_at_ms ? new Date(sprint.starts_at_ms).toLocaleDateString() : "No start"}</span><span>{sprint.ends_at_ms ? new Date(sprint.ends_at_ms).toLocaleDateString() : "No end"}</span></footer></article>
              {:else}<div class="record-empty"><Gauge size={28} /><h2>Plan the next sprint</h2><p>Time-box a clear delivery goal for the team and its agents.</p><button class="secondary-button" onclick={openFeatureModal} type="button">Create sprint</button></div>{/each}
            {:else if activeView === "modules"}
              {#each projectModules as module (module.id)}
                <article><div><Layers3 size={17} /><span class="status-badge">{module.status.replaceAll("_", " ")}</span></div><h2>{module.name}</h2><p>{module.description || "No module brief yet."}</p><footer><span>Target</span><span>{module.target_at_ms ? new Date(module.target_at_ms).toLocaleDateString() : "Open"}</span></footer></article>
              {:else}<div class="record-empty"><Layers3 size={28} /><h2>Group work into a module</h2><p>Give a larger product outcome a durable home.</p><button class="secondary-button" onclick={openFeatureModal} type="button">Create module</button></div>{/each}
            {:else if activeView === "pages"}
              {#each projectPages as page (page.id)}
                <article><div><FileText size={17} /><span>v{page.version}</span></div><h2>{page.title}</h2><p>{page.content || "Empty page"}</p><footer><span>Project knowledge</span><span>SQLite</span></footer></article>
              {:else}<div class="record-empty"><FileText size={28} /><h2>Write the first page</h2><p>Keep product context close to the work it informs.</p><button class="secondary-button" onclick={openFeatureModal} type="button">Create page</button></div>{/each}
            {:else}
              {#each projectIntake as item (item.id)}
                <article><div><Inbox size={17} /><span class="status-badge">{item.status}</span></div><h2>{item.title}</h2><p>{item.description || "No additional context."}</p><footer><span>{item.submitter_email ?? "Internal"}</span><span>{item.ticket_id ? "Accepted" : "Untriaged"}</span></footer></article>
              {:else}<div class="record-empty"><Inbox size={28} /><h2>Capture incoming work</h2><p>Collect requests before committing them to delivery.</p><button class="secondary-button" onclick={openFeatureModal} type="button">Create intake item</button></div>{/each}
            {/if}
          </div>
        </section>
      {/if}

      {#if selectedTicket}
        <button
          aria-label="Close ticket details"
          class="drawer-scrim"
          onclick={() => (selectedTicketId = null)}
          type="button"
        ></button>
        <aside aria-label="Ticket details" class="ticket-drawer">
          <header>
            <div><span class="ticket-key">{ticketKey(selectedTicket)}</span><h2>{selectedTicket.title}</h2></div>
            <button aria-label="Close ticket details" class="icon-button" onclick={() => (selectedTicketId = null)} type="button"><X size={17} /></button>
          </header>
          <div class="drawer-body">
            {#if selectedTicket.description}<p class="ticket-description">{selectedTicket.description}</p>{/if}
            <div class="ticket-meta">
              <span><small>PRIORITY</small>{priorityLabel(selectedTicket.priority)}</span>
              <span><small>VERSION</small>{selectedTicket.version}</span>
            </div>
            <label class="ticket-state-control"><span>Workflow state</span><select disabled={submitting} onchange={(event) => moveTicket(event.currentTarget.value)} value={ticketStateGroup(selectedTicket)}><option value="backlog">Backlog</option><option value="unstarted">Todo</option><option value="started">In progress</option><option value="completed">Done</option><option value="cancelled">Cancelled</option></select></label>
            <section class="agent-panel">
              <div class="agent-panel-title"><div><Bot size={17} /><span><small>EXECUTION</small><strong>Agent run</strong></span></div>{#if runSnapshot}<span class={`run-state state-${runSnapshot.run.state}`}>{runSnapshot.run.state.replaceAll("_", " ")}</span>{/if}</div>
              {#if executionLoading && !executionBootstrap}
                <p class="panel-note">Loading execution capabilities…</p>
              {:else if !runSnapshot}
                <p class="panel-note">Create an isolated changeset and let a local provider work against this ticket under supervision.</p>
                <label><span>Repository</span><select bind:value={selectedRepositoryId} disabled={approvedRepositories.length === 0}><option value="">{approvedRepositories.length === 0 ? "No approved repositories" : "Select repository"}</option>{#each approvedRepositories as repository (repository.id)}<option value={repository.id}>{repository.display_name}</option>{/each}</select></label>
                <label><span>Provider</span><select bind:value={selectedProvider}>{#each executionBootstrap?.provider_availability ?? ["mock"] as provider (provider)}<option value={provider}>{provider}</option>{/each}</select></label>
                <button class="primary-button" disabled={executionLoading || !selectedRepositoryId} onclick={startTicketRun} type="button"><Sparkles size={15} /> Start agent run</button>
                {#if approvedRepositories.length === 0}<p class="repo-hint">Start the instance with <code>JET_BLACK_REPOSITORY_ROOTS=/absolute/repository</code> to approve local execution.</p>{/if}
              {:else}
                <div class="run-timeline">
                  <span class:complete={true}><i><Check size={12} /></i><b>Changeset created</b><small>{runSnapshot.changeset.id.slice(0, 8)}</small></span>
                  <span class:complete={!["queued", "starting"].includes(runSnapshot.run.state)}><i>{#if !["queued", "starting"].includes(runSnapshot.run.state)}<Check size={12} />{:else}<CircleDot size={12} />{/if}</i><b>Provider execution</b><small>{runSnapshot.run.provider_selection?.kind ?? "provider"}</small></span>
                  <span class:complete={Boolean(runSnapshot.pending_approval) || ["completed", "reviewable"].includes(runSnapshot.run.state)}><i><CircleDot size={12} /></i><b>Scoped approval</b><small>Exact revision</small></span>
                </div>
                {#if runSnapshot.pending_approval}
                  <div class="approval-card">
                    <p class="eyebrow">APPROVAL REQUIRED</p>
                    <strong>{runSnapshot.pending_approval.scope.proposal.action.replaceAll("_", " ")}</strong>
                    <code>{runSnapshot.pending_approval.scope.proposal.target_path}</code>
                    <div><button class="ghost-button" disabled={executionLoading} onclick={() => respondToApproval(false)} type="button">Reject</button><button class="primary-button compact" disabled={executionLoading} onclick={() => respondToApproval(true)} type="button"><Check size={14} /> Approve exact change</button></div>
                  </div>
                {/if}
                {#if ["queued", "starting", "running", "awaiting_approval"].includes(runSnapshot.run.state)}
                  <button class="danger-button" disabled={executionLoading} onclick={interruptRun} type="button">Interrupt run</button>
                {/if}
                {#if runSnapshot.changeset.state === "reviewable"}
                  <button class="secondary-button" disabled={executionLoading} onclick={reviewChanges} type="button"><Search size={14} /> Run local review</button>
                {/if}
                {#if reviewReport}
                  <div class="review-summary">
                    <p class="eyebrow">REVIEW REPORT</p>
                    <div>{#each reviewReport.checks as check (check.kind)}<span class:passed={check.status === "passed"}><b>{check.kind.replaceAll("_", " ")}</b><small>{check.status}</small></span>{/each}</div>
                    <p>{reviewReport.findings.length} findings · {reviewReport.changed_paths.length} changed files</p>
                  </div>
                {/if}
              {/if}
            </section>
          </div>
        </aside>
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
        <label><span>Repository</span><select bind:value={repositoryKind}><option value="none">No repository yet</option><option value="local">Local Git repository</option><option value="remote">Remote repository</option></select></label>
        {#if repositoryKind === "local"}
          <label><span>Path on this device</span><input bind:value={repositoryLocation} placeholder="/home/you/code/project" required /></label>
          <p class="field-hint"><GitBranch size={13} /> The origin remote will be detected from Git.</p>
        {:else if repositoryKind === "remote"}
          <label><span>Repository URL</span><input bind:value={repositoryLocation} placeholder="https://github.com/org/repository.git" required type="url" /></label>
        {/if}
      {:else if modal === "connection"}
        <label><span>Connection name</span><input bind:value={connectionName} placeholder="Company control plane" required /></label>
        <label><span>Control plane URL</span><input bind:value={connectionUrl} placeholder="https://jet-black.example.com" required type="url" /></label>
        <label><span>One-time access token</span><input autocomplete="off" bind:value={connectionToken} placeholder="Paste token from account settings" required /></label>
      {:else if modal === "ticket"}
        <label><span>Title</span><Input bind:value={ticketTitle} placeholder="What needs to ship?" required /></label>
        <label><span>Description</span><Textarea bind:value={ticketDescription} placeholder="Give the team and agents enough context to act."></Textarea></label>
        <label><span>Priority</span><select bind:value={ticketPriority}><option value="none">No priority</option><option value="urgent">Urgent</option><option value="high">High</option><option value="medium">Medium</option><option value="low">Low</option></select></label>
      {:else}
        <label><span>{modal === "page" ? "Title" : "Name"}</span><input bind:value={recordName} placeholder={modal === "page" ? "Architecture notes" : "Platform reliability"} required /></label>
        {#if modal === "page"}
          <label><span>Content</span><textarea bind:value={recordContent} placeholder="Write durable project context…"></textarea></label>
        {:else}
          <label><span>Description</span><textarea bind:value={recordDescription} placeholder="What should the team know?"></textarea></label>
        {/if}
        {#if modal === "sprint"}
          <div class="field-row"><label><span>Starts</span><input bind:value={recordStartDate} type="date" /></label><label><span>Ends</span><input bind:value={recordEndDate} type="date" /></label></div>
        {:else if modal === "module"}
          <label><span>Target date</span><input bind:value={recordTargetDate} type="date" /></label>
        {:else if modal === "intake"}
          <label><span>Submitter email <small>optional</small></span><input bind:value={recordEmail} placeholder="requester@example.com" type="email" /></label>
        {/if}
      {/if}
      <footer><button class="ghost-button" onclick={resetModal} type="button">Cancel</button><button class="primary-button compact" disabled={submitting} type="submit">{submitting ? "Creating…" : `Create ${modal}`}</button></footer>
    </form>
  </div>
{/if}
</div>
