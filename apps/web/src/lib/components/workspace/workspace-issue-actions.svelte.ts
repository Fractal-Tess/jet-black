import { api } from "@workspace/convex/api";
import { useMutation } from "convex-svelte";
import { goto } from "$app/navigation";
import type {
  AddAttachmentInput,
  CreateIssueInput,
  CreateLabelInput,
  Issue,
  IssueLabel,
  IssueState,
  UpdateIssueInput,
} from "$lib/components/issues/types";
import { issueHref, projectModuleHref } from "$lib/routes";
import type { WorkspaceActionDeps } from "./workspace-action-deps";

export function createWorkspaceIssueActions(deps: WorkspaceActionDeps) {
  const createIssue = useMutation(api.mutations.issues.create);
  const updateIssue = useMutation(api.mutations.issues.update);
  const moveIssue = useMutation(api.mutations.issues.move);
  const archiveIssue = useMutation(api.mutations.issues.archive);
  const addAttachment = useMutation(api.mutations.attachments.addLink);
  const createComment = useMutation(api.mutations.comments.create);
  const createLabel = useMutation(api.mutations.labels.create);
  const toggleIssueLabel = useMutation(api.mutations.labels.toggleForIssue);

  let creating = $state(false);

  async function onCreateIssue(input: CreateIssueInput) {
    const activeProject = deps.getActiveProject();

    if (!activeProject) {
      return;
    }

    creating = true;

    try {
      const issueId = await createIssue({
        ...input,
        projectId: activeProject._id,
      });
      deps.setSelectedIssueId(issueId);

      const viewerData = deps.getViewerData();

      if (viewerData?.activeWorkspace) {
        await goto(
          issueHref({
            issueId,
            projectId: activeProject._id,
            workspaceSlug: viewerData.activeWorkspace.slug,
          })
        );
      }
    } finally {
      creating = false;
    }
  }

  async function onQuickCreateIssue(state: IssueState, title: string) {
    await onCreateIssue({
      priority: "medium",
      stateId: state._id,
      title,
    });
  }

  async function onCreateSubIssue(title: string) {
    const selectedIssue = deps.getSelectedIssue();

    if (!selectedIssue) {
      return;
    }

    await createIssue({
      parentIssueId: selectedIssue._id,
      priority: "medium",
      projectId: selectedIssue.projectId,
      stateId: selectedIssue.stateId,
      title,
    });
  }

  async function onUpdateIssue(input: UpdateIssueInput) {
    const selectedIssue = deps.getSelectedIssue();

    if (!selectedIssue) {
      return;
    }

    await updateIssue({
      ...input,
      issueId: selectedIssue._id,
    });
  }

  async function onUpdateIssueFor(issue: Issue, input: UpdateIssueInput) {
    await updateIssue({
      ...input,
      issueId: issue._id,
    });
  }

  async function onAddComment(body: string) {
    const selectedIssue = deps.getSelectedIssue();

    if (!selectedIssue) {
      return;
    }
    await createComment({
      body,
      issueId: selectedIssue._id,
    });
  }

  async function onAddAttachment(input: AddAttachmentInput) {
    const selectedIssue = deps.getSelectedIssue();

    if (!selectedIssue) {
      return;
    }
    await addAttachment({
      issueId: selectedIssue._id,
      name: input.name,
      url: input.url,
    });
  }

  async function onCreateLabel(input: CreateLabelInput) {
    const activeProject = deps.getActiveProject();

    if (!activeProject) {
      return;
    }
    await createLabel({
      ...input,
      projectId: activeProject._id,
    });
  }

  async function onToggleLabel(labelId: IssueLabel["_id"]) {
    const selectedIssue = deps.getSelectedIssue();

    if (!selectedIssue) {
      return;
    }
    await toggleIssueLabel({
      issueId: selectedIssue._id,
      labelId,
    });
  }

  async function onMoveIssue(
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) {
    if (issue.stateId === stateId && issue.position === position) {
      return;
    }
    await moveIssue({
      issueId: issue._id,
      position,
      stateId,
    });
  }

  async function onArchiveIssue() {
    const activeProject = deps.getActiveProject();
    const selectedIssue = deps.getSelectedIssue();

    if (!(activeProject && selectedIssue)) {
      return;
    }

    await archiveIssue({
      issueId: selectedIssue._id,
    });
    deps.setSelectedIssueId(undefined);

    const viewerData = deps.getViewerData();

    if (viewerData?.activeWorkspace) {
      await goto(
        projectModuleHref({
          module: "tickets",
          projectId: activeProject._id,
          workspaceSlug: viewerData.activeWorkspace.slug,
        })
      );
    }
  }

  return {
    get creating() {
      return creating;
    },
    onAddAttachment,
    onAddComment,
    onArchiveIssue,
    onCreateIssue,
    onCreateLabel,
    onCreateSubIssue,
    onMoveIssue,
    onQuickCreateIssue,
    onReorderIssue: onMoveIssue,
    onToggleLabel,
    onUpdateIssue,
    onUpdateIssueFor,
  };
}
