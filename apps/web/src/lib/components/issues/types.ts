import type { Id } from "../../../../../../convex/convex/_generated/dataModel";

export type IssuePriority = "none" | "low" | "medium" | "high" | "urgent";

export type IssueState = {
  _id: Id<"issueStates">;
  color: string;
  isDefault: boolean;
  name: string;
  position: number;
  type: "backlog" | "unstarted" | "started" | "completed" | "cancelled";
};

export type IssueLabel = {
  _id: Id<"issueLabels">;
  color: string;
  name: string;
  projectId: Id<"projects">;
  workspaceId: Id<"workspaces">;
};

export type Issue = {
  _creationTime: number;
  _id: Id<"issues">;
  description?: string;
  identifier: string;
  labels: IssueLabel[];
  position?: number;
  priority: IssuePriority;
  state: IssueState | null;
  stateId: Id<"issueStates">;
  title: string;
  updatedAt: number;
};

export type IssueComment = {
  _creationTime: number;
  _id: Id<"issueComments">;
  authorUserId: string;
  body: string;
  updatedAt: number;
};

export type Project = {
  _id: Id<"projects">;
  color: string;
  description?: string;
  key: string;
  name: string;
};

export type ViewerData = {
  activeProject: Project | null;
  activeWorkspace: {
    _id: Id<"workspaces">;
    name: string;
    slug: string;
  } | null;
  projects: Project[];
  user: {
    email: string;
    id: string;
    image: string | null;
    name: string;
  };
};
