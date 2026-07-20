<script lang="ts">
import type { IssueDisplayOptions } from "$lib/components/issues/display-options";
import type {
  AddAttachmentInput,
  AddModuleLinkInput,
  CreateIntakeIssueInput,
  CreateIssueInput,
  CreateLabelInput,
  CreateProjectModuleInput,
  CreateProjectPageInput,
  CreateSprintInput,
  IntakeIssue,
  Issue,
  IssueActivity,
  IssueAttachment,
  IssueComment,
  IssueLabel,
  IssueState,
  ModuleDetail,
  ModuleLink,
  Project,
  ProjectModuleRecord,
  ProjectPage,
  Sprint,
  UpdateIssueInput,
  UpdateModuleLinkInput,
  UpdateProjectModuleInput,
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
  issues,
  sprints,
  modules,
  archivedModules,
  moduleDetail,
  moduleDetailLoading,
  routeModuleId,
  pages,
  activities,
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
  onUpdateModule,
  onArchiveModule,
  onRestoreModule,
  onDeleteModule,
  onAssignIssueToModule,
  onRemoveIssueFromModule,
  onCreateModuleIssue,
  onAddModuleLink,
  onUpdateModuleLink,
  onRemoveModuleLink,
  onCreatePage,
  onUpdatePage,
  onAddAttachment,
  onAddComment,
  onUpdateComment,
  onDeleteComment,
  onCreateLabel,
  onCreateModuleForIssue,
  onCreateSubIssue,
  onArchiveIssue,
  onMoveIssue,
  onOpenCreateIssue,
  onQuickCreateIssue,
  onReorderIssue,
  onSelectIssue,
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
  issues: Issue[];
  sprints: Sprint[];
  modules: ProjectModuleRecord[];
  archivedModules: ProjectModuleRecord[];
  moduleDetail: ModuleDetail | null;
  moduleDetailLoading: boolean;
  routeModuleId: string | undefined;
  pages: ProjectPage[];
  activities: IssueActivity[];
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
  onUpdateModule: (
    moduleId: ProjectModuleRecord["_id"],
    input: UpdateProjectModuleInput
  ) => Promise<void>;
  onArchiveModule: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onRestoreModule: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onDeleteModule: (moduleId: ProjectModuleRecord["_id"]) => Promise<void>;
  onAssignIssueToModule: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onRemoveIssueFromModule: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onCreateModuleIssue: (
    moduleId: ProjectModuleRecord["_id"],
    input: CreateIssueInput
  ) => Promise<void>;
  onAddModuleLink: (
    moduleId: ProjectModuleRecord["_id"],
    input: AddModuleLinkInput
  ) => Promise<void>;
  onUpdateModuleLink: (
    linkId: ModuleLink["_id"],
    input: UpdateModuleLinkInput
  ) => Promise<void>;
  onRemoveModuleLink: (linkId: ModuleLink["_id"]) => Promise<void>;
  onCreatePage: (input: CreateProjectPageInput) => Promise<void>;
  onUpdatePage: (
    pageId: ProjectPage["_id"],
    input: UpdateProjectPageInput
  ) => Promise<void>;
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onAddComment: (
    body: string,
    parentCommentId?: IssueComment["_id"]
  ) => Promise<void>;
  onUpdateComment: (
    commentId: IssueComment["_id"],
    body: string
  ) => Promise<void>;
  onDeleteComment: (commentId: IssueComment["_id"]) => Promise<void>;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateModuleForIssue: (name: string) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onOpenCreateIssue?: (stateId: IssueState["_id"]) => void;
  onQuickCreateIssue: (state: IssueState, title: string) => Promise<void>;
  onReorderIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onSelectIssue: (issueId: Issue["_id"]) => void;
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
    {activeProject}
    creating={creatingModule}
    {issues}
    {modules}
    {archivedModules}
    {moduleDetail}
    {moduleDetailLoading}
    routeModuleId={routeModuleId ?? undefined}
    {workspaceSlug}
    members={workspaceMembers}
    {states}
    {activities}
    {attachments}
    comments={comments}
    {labels}
    {selectedIssueId}
    selectedSubIssues={selectedSubIssues}
    onAddAttachment={onAddAttachment}
    onAddComment={onAddComment}
    onUpdateComment={onUpdateComment}
    onDeleteComment={onDeleteComment}
    onCreateLabel={onCreateLabel}
    onCreateModuleForIssue={onCreateModuleForIssue}
    onCreateSubIssue={onCreateSubIssue}
    onArchiveIssue={onArchiveIssue}
    onSelectIssue={onSelectIssue}
    onToggleLabel={onToggleLabel}
    onUpdateIssue={onUpdateIssue}
    onArchive={onArchiveModule}
    onCreate={onCreateModule}
    onCreateIssue={onCreateModuleIssue}
    onDelete={onDeleteModule}
    onAddLink={onAddModuleLink}
    onUpdateLink={onUpdateModuleLink}
    onRemoveLink={onRemoveModuleLink}
    onRemoveIssue={onRemoveIssueFromModule}
    onRestore={onRestoreModule}
    onUpdate={onUpdateModule}
    onAssignIssue={onAssignIssueToModule}
    onMoveIssue={onMoveIssue}
    onReorderIssue={onReorderIssue}
    onUpdateIssueFor={onUpdateIssueFor}
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
    {activities}
    {attachments}
    comments={comments}
    {displayOptions}
    {filteredIssues}
    {issueView}
    {labels}
    members={workspaceMembers}
    {modules}
    onAddAttachment={onAddAttachment}
    onAddComment={onAddComment}
    onUpdateComment={onUpdateComment}
    onDeleteComment={onDeleteComment}
    onCreateLabel={onCreateLabel}
    onCreateModuleForIssue={onCreateModuleForIssue}
    onCreateSubIssue={onCreateSubIssue}
    onArchiveIssue={onArchiveIssue}
    onMoveIssue={onMoveIssue}
    onOpenCreateIssue={onOpenCreateIssue}
    onQuickCreateIssue={onQuickCreateIssue}
    onReorderIssue={onReorderIssue}
    onSelectIssue={onSelectIssue}
    onToggleLabel={onToggleLabel}
    onUpdateIssue={onUpdateIssue}
    onUpdateIssueFor={onUpdateIssueFor}
    {selectedIssueId}
    selectedSubIssues={selectedSubIssues}
    {states}
    {workspaceSlug}
  />
{/if}
