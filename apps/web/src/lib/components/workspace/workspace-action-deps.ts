import type { Issue, Project, ViewerData } from "$lib/components/issues/types";

export type WorkspaceActionDeps = {
  getActiveProject: () => Project | null;
  getSelectedIssue: () => Issue | null;
  getViewerData: () => ViewerData | null;
  setSelectedIssueId: (issueId: Issue["_id"] | undefined) => void;
  setSelectedProjectId: (projectId: Project["_id"] | undefined) => void;
};
