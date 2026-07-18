import { api } from "@workspace/convex/api";
import { useMutation } from "convex-svelte";
import { goto } from "$app/navigation";
import type {
  CreateIntakeIssueInput,
  CreateProjectModuleInput,
  CreateProjectPageInput,
  CreateSprintInput,
  IntakeIssue,
  Issue,
  ProjectModuleRecord,
  ProjectPage,
  Sprint,
  UpdateProjectPageInput,
} from "$lib/components/issues/types";
import { issueHref } from "$lib/routes";
import type { WorkspaceActionDeps } from "./workspace-action-deps";

export function createWorkspaceProjectModuleActions(deps: WorkspaceActionDeps) {
  const updateIssue = useMutation(api.mutations.issues.update);
  const acceptIntakeIssue = useMutation(api.mutations.intake.accept);
  const createIntakeIssue = useMutation(api.mutations.intake.create);
  const updateIntakeStatus = useMutation(api.mutations.intake.updateStatus);
  const createProjectModule = useMutation(api.mutations.modules.create);
  const createPage = useMutation(api.mutations.pages.create);
  const updatePage = useMutation(api.mutations.pages.update);
  const createSprint = useMutation(api.mutations.sprints.create);

  let creatingIntake = $state(false);
  let creatingModule = $state(false);
  let creatingPage = $state(false);
  let creatingSprint = $state(false);

  async function onCreateIntakeIssue(input: CreateIntakeIssueInput) {
    const activeProject = deps.getActiveProject();

    if (!activeProject) {
      return;
    }

    creatingIntake = true;

    try {
      await createIntakeIssue({
        ...input,
        projectId: activeProject._id,
      });
    } finally {
      creatingIntake = false;
    }
  }

  async function onAcceptIntakeIssue(intakeIssue: IntakeIssue) {
    const issueId = await acceptIntakeIssue({
      intakeIssueId: intakeIssue._id,
    });
    const viewerData = deps.getViewerData();
    const activeProject = deps.getActiveProject();

    if (issueId && viewerData?.activeWorkspace && activeProject) {
      deps.setSelectedIssueId(issueId);
      await goto(
        issueHref({
          issueId,
          projectId: activeProject._id,
          workspaceSlug: viewerData.activeWorkspace.slug,
        })
      );
    }
  }

  async function onDeclineIntakeIssue(intakeIssue: IntakeIssue) {
    await updateIntakeStatus({
      intakeIssueId: intakeIssue._id,
      status: "declined",
    });
  }

  async function onCreateSprint(input: CreateSprintInput) {
    const activeProject = deps.getActiveProject();

    if (!activeProject) {
      return;
    }

    creatingSprint = true;

    try {
      await createSprint({
        ...input,
        projectId: activeProject._id,
      });
    } finally {
      creatingSprint = false;
    }
  }

  async function onCreateModule(input: CreateProjectModuleInput) {
    const activeProject = deps.getActiveProject();

    if (!activeProject) {
      return;
    }
    creatingModule = true;

    try {
      await createProjectModule({
        ...input,
        projectId: activeProject._id,
      });
    } finally {
      creatingModule = false;
    }
  }

  async function onCreatePage(input: CreateProjectPageInput) {
    const activeProject = deps.getActiveProject();

    if (!activeProject) {
      return;
    }
    creatingPage = true;

    try {
      await createPage({
        ...input,
        projectId: activeProject._id,
      });
    } finally {
      creatingPage = false;
    }
  }

  async function onUpdatePage(
    pageId: ProjectPage["_id"],
    input: UpdateProjectPageInput
  ) {
    await updatePage({
      ...input,
      pageId,
    });
  }

  async function onAssignIssueToModule(
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) {
    await updateIssue({
      issueId,
      moduleId,
    });
  }

  async function onAssignIssueToSprint(
    issueId: Issue["_id"],
    sprintId: Sprint["_id"]
  ) {
    await updateIssue({
      issueId,
      sprintId,
    });
  }

  return {
    get creatingIntake() {
      return creatingIntake;
    },
    get creatingModule() {
      return creatingModule;
    },
    get creatingPage() {
      return creatingPage;
    },
    get creatingSprint() {
      return creatingSprint;
    },
    onAcceptIntakeIssue,
    onAssignIssueToModule,
    onAssignIssueToSprint,
    onCreateIntakeIssue,
    onCreateModule,
    onCreatePage,
    onCreateSprint,
    onDeclineIntakeIssue,
    onUpdatePage,
  };
}
