<script lang="ts">
import type { IssueDisplayOptions } from "$lib/components/issues/display-options";
import type {
  AddAttachmentInput,
  CreateIntakeIssueInput,
  CreateLabelInput,
  CreateProjectModuleInput,
  CreateProjectPageInput,
  CreateSprintInput,
  IntakeIssue,
  Issue,
  IssueAttachment,
  IssueComment,
  IssueLabel,
  IssueState,
  Project,
  ProjectModuleRecord,
  ProjectPage,
  Sprint,
  UpdateIssueInput,
  UpdateProjectPageInput,
  WorkspaceMember,
} from "$lib/components/issues/types";
import IntakeModule from "$lib/components/projects/IntakeModule.svelte";
import ModulesModule from "$lib/components/projects/ModulesModule.svelte";
import PagesModule from "$lib/components/projects/PagesModule.svelte";
import ProjectModulePlaceholder from "$lib/components/projects/ProjectModulePlaceholder.svelte";
import SprintsModule from "$lib/components/projects/SprintsModule.svelte";
import type { ProjectModule } from "$lib/routes";
import TicketsView from "../../../routes/workspace/[workspaceSlug]/projects/[projectId]/[module]/components/TicketsView.svelte";

type PlaceholderProjectModule = Exclude<ProjectModule, "issues" | "tickets">;

let {
  activeModule,
  activePlaceholderModule,
  activeProject,
  workspaceSlug,
  creatingIntake,
  creatingSprint,
  creatingModule,
  creatingPage,
  intakeIssues,
  filteredIssues,
  sprints,
  modules,
  pages,
  attachments,
  comments,
  displayOptions,
  issueView,
  labels,
  workspaceMembers,
  selectedIssueId,
  selectedSubIssues,
  states,
  onAcceptIntakeIssue,
  onCreateIntakeIssue,
  onDeclineIntakeIssue,
  onCreateSprint,
  onAssignIssueToSprint,
  onCreateModule,
  onAssignIssueToModule,
  onCreatePage,
  onUpdatePage,
  onAddAttachment,
  onAddComment,
  onCreateLabel,
  onCreateSubIssue,
  onArchiveIssue,
  onMoveIssue,
  onQuickCreateIssue,
  onReorderIssue,
  onToggleLabel,
  onUpdateIssue,
  onUpdateIssueFor,
}: {
  activeModule: ProjectModule;
  activePlaceholderModule: PlaceholderProjectModule | null;
  activeProject: Project;
  workspaceSlug: string;
  creatingIntake: boolean;
  creatingSprint: boolean;
  creatingModule: boolean;
  creatingPage: boolean;
  intakeIssues: IntakeIssue[];
  filteredIssues: Issue[];
  sprints: Sprint[];
  modules: ProjectModuleRecord[];
  pages: ProjectPage[];
  attachments: IssueAttachment[];
  comments: IssueComment[];
  displayOptions: IssueDisplayOptions;
  issueView: "board" | "list";
  labels: IssueLabel[];
  workspaceMembers: WorkspaceMember[];
  selectedIssueId: Issue["_id"] | undefined;
  selectedSubIssues: Issue[];
  states: IssueState[];
  onAcceptIntakeIssue: (intakeIssue: IntakeIssue) => Promise<void>;
  onCreateIntakeIssue: (input: CreateIntakeIssueInput) => Promise<void>;
  onDeclineIntakeIssue: (intakeIssue: IntakeIssue) => Promise<void>;
  onCreateSprint: (input: CreateSprintInput) => Promise<void>;
  onAssignIssueToSprint: (
    issueId: Issue["_id"],
    sprintId: Sprint["_id"]
  ) => Promise<void>;
  onCreateModule: (input: CreateProjectModuleInput) => Promise<void>;
  onAssignIssueToModule: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onCreatePage: (input: CreateProjectPageInput) => Promise<void>;
  onUpdatePage: (
    pageId: ProjectPage["_id"],
    input: UpdateProjectPageInput
  ) => Promise<void>;
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onAddComment: (body: string) => Promise<void>;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onQuickCreateIssue: (state: IssueState, title: string) => Promise<void>;
  onReorderIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
  onUpdateIssueFor: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
} = $props();
</script>

{#if activeModule === "intake"}
  <IntakeModule
    creating={creatingIntake}
    {intakeIssues}
    onAccept={onAcceptIntakeIssue}
    onCreate={onCreateIntakeIssue}
    onDecline={onDeclineIntakeIssue}
  />
{:else if activeModule === "sprints"}
  <SprintsModule
    creating={creatingSprint}
    issues={filteredIssues}
    onAssignIssue={onAssignIssueToSprint}
    onCreate={onCreateSprint}
    {sprints}
  />
{:else if activeModule === "modules"}
  <ModulesModule
    creating={creatingModule}
    issues={filteredIssues}
    {modules}
    onAssignIssue={onAssignIssueToModule}
    onCreate={onCreateModule}
  />
{:else if activeModule === "pages"}
  <PagesModule
    creating={creatingPage}
    onCreate={onCreatePage}
    onUpdate={onUpdatePage}
    {pages}
  />
{:else if activePlaceholderModule}
  <ProjectModulePlaceholder
    module={activePlaceholderModule}
    projectName={activeProject.name}
  />
{:else}
  <TicketsView
    {activeProject}
    {attachments}
    comments={comments}
    {displayOptions}
    {filteredIssues}
    {issueView}
    {labels}
    members={workspaceMembers}
    onAddAttachment={onAddAttachment}
    onAddComment={onAddComment}
    onCreateLabel={onCreateLabel}
    onCreateSubIssue={onCreateSubIssue}
    onArchiveIssue={onArchiveIssue}
    onMoveIssue={onMoveIssue}
    onQuickCreateIssue={onQuickCreateIssue}
    onReorderIssue={onReorderIssue}
    onToggleLabel={onToggleLabel}
    onUpdateIssue={onUpdateIssue}
    onUpdateIssueFor={onUpdateIssueFor}
    {selectedIssueId}
    selectedSubIssues={selectedSubIssues}
    {states}
    {workspaceSlug}
  />
{/if}
