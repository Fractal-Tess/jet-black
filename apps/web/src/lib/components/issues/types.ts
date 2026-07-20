import type { api } from "@workspace/convex/api";
import type { FunctionArgs, FunctionReturnType } from "convex/server";

export type ViewerQueryResult = FunctionReturnType<
  typeof api.queries.workspaces.viewer
>;

export type ViewerData = ViewerQueryResult;

export type Project = ViewerData["projects"][number];

export type WorkspaceUser = Pick<
  ViewerData["user"],
  "email" | "image" | "name"
>;

export type IssuesQueryResult = FunctionReturnType<
  typeof api.queries.issues.listForProject
>;

export type Issue = IssuesQueryResult[number];

export type IssuePriority = Issue["priority"];

export type IssueStatesQueryResult = FunctionReturnType<
  typeof api.queries.workspaces.statesForProject
>;

export type IssueState = IssueStatesQueryResult[number];

export type IssueLabelsQueryResult = FunctionReturnType<
  typeof api.queries.labels.listForProject
>;

export type IssueLabel = IssueLabelsQueryResult[number];

export type Sprint = FunctionReturnType<
  typeof api.queries.sprints.listForProject
>[number];

export type ProjectModuleRecord = FunctionReturnType<
  typeof api.queries.modules.listForProject
>[number];

export type ProjectPage = FunctionReturnType<
  typeof api.queries.pages.listForProject
>[number];

export type IssueComment = FunctionReturnType<
  typeof api.queries.comments.listForIssue
>[number];

export type IssueActivity = FunctionReturnType<
  typeof api.queries.activities.listForIssue
>[number];

export type IssueAttachment = FunctionReturnType<
  typeof api.queries.attachments.listForIssue
>[number];

export type IntakeIssue = FunctionReturnType<
  typeof api.queries.intake.listForProject
>[number];

export type WorkspaceMember = FunctionReturnType<
  typeof api.queries.workspaces.membersForWorkspace
>[number];

export type DashboardOverview = FunctionReturnType<
  typeof api.queries.dashboard.overviewForWorkspace
>;

export type WorkspaceAnalyticsData = FunctionReturnType<
  typeof api.queries.dashboard.analyticsForWorkspace
>;

export type AddAttachmentInput = Omit<
  FunctionArgs<typeof api.mutations.attachments.addLink>,
  "issueId"
>;

export type CreateLabelInput = Omit<
  FunctionArgs<typeof api.mutations.labels.create>,
  "projectId"
>;

export type UpdateIssueInput = Omit<
  FunctionArgs<typeof api.mutations.issues.update>,
  "issueId"
>;

export type CreateIssueInput = Omit<
  FunctionArgs<typeof api.mutations.issues.create>,
  "projectId"
>;

export type CreateIntakeIssueInput = Omit<
  FunctionArgs<typeof api.mutations.intake.create>,
  "projectId"
>;

export type CreateProjectInput = Omit<
  FunctionArgs<typeof api.mutations.projects.create>,
  "workspaceId"
>;

export type CreateProjectModuleInput = Omit<
  FunctionArgs<typeof api.mutations.modules.create>,
  "projectId"
>;

export type CreateProjectPageInput = Omit<
  FunctionArgs<typeof api.mutations.pages.create>,
  "projectId"
>;

export type UpdateProjectPageInput = Omit<
  FunctionArgs<typeof api.mutations.pages.update>,
  "pageId"
>;

export type CreateSprintInput = Omit<
  FunctionArgs<typeof api.mutations.sprints.create>,
  "projectId"
>;

export type ModuleDetail = FunctionReturnType<typeof api.queries.modules.get>;

export type ModuleLink = NonNullable<ModuleDetail>["links"][number];

export type UpdateProjectModuleInput = Omit<
  FunctionArgs<typeof api.mutations.modules.update>,
  "moduleId"
>;

export type AddModuleLinkInput = Omit<
  FunctionArgs<typeof api.mutations.modules.addLink>,
  "moduleId"
>;

export type UpdateModuleLinkInput = Omit<
  FunctionArgs<typeof api.mutations.modules.updateLink>,
  "linkId"
>;

export type ModuleIssueAssignmentInput = Omit<
  FunctionArgs<typeof api.mutations.modules.addIssueAssignment>,
  "moduleId"
>;
