import { api } from "@workspace/convex/api";
import { useMutation } from "convex-svelte";
import { goto } from "$app/navigation";
import type {
  AddModuleLinkInput,
  CreateIntakeIssueInput,
  CreateIssueInput,
  CreateProjectModuleInput,
  CreateProjectPageInput,
  CreateSprintInput,
  IntakeIssue,
  Issue,
  ModuleLink,
  ProjectModuleRecord,
  ProjectPage,
  Sprint,
  UpdateModuleLinkInput,
  UpdateProjectModuleInput,
  UpdateProjectPageInput,
} from "$lib/components/issues/types";
import { issueHref, projectModuleHref } from "$lib/routes";
import type { WorkspaceActionDeps } from "./workspace-action-deps";

export function createWorkspaceProjectModuleActions(deps: WorkspaceActionDeps) {
  const updateIssue = useMutation(api.mutations.issues.update);
  const createIssue = useMutation(api.mutations.issues.create);
  const acceptIntakeIssue = useMutation(api.mutations.intake.accept);
  const createIntakeIssue = useMutation(api.mutations.intake.create);
  const updateIntakeStatus = useMutation(api.mutations.intake.updateStatus);
  const createProjectModule = useMutation(api.mutations.modules.create);
  const updateProjectModule = useMutation(api.mutations.modules.update);
  const archiveProjectModule = useMutation(api.mutations.modules.archive);
  const restoreProjectModule = useMutation(api.mutations.modules.restore);
  const removeProjectModule = useMutation(api.mutations.modules.remove);
  const addModuleLink = useMutation(api.mutations.modules.addLink);
  const updateModuleLinkMutation = useMutation(
    api.mutations.modules.updateLink
  );
  const removeModuleLinkMutation = useMutation(
    api.mutations.modules.removeLink
  );
  const addIssueAssignment = useMutation(
    api.mutations.modules.addIssueAssignment
  );
  const removeIssueAssignment = useMutation(
    api.mutations.modules.removeIssueAssignment
  );
  const createPage = useMutation(api.mutations.pages.create);
  const updatePage = useMutation(api.mutations.pages.update);
  const createSprint = useMutation(api.mutations.sprints.create);

  let creatingIntake = $state(false);
  let creatingModule = $state(false);
  let creatingModuleIssue = $state(false);
  let creatingPage = $state(false);
  let creatingSprint = $state(false);
  let updatingModule = $state(false);
  let archivingModule = $state(false);
  let restoringModule = $state(false);
  let deletingModule = $state(false);
  let addingModuleLink = $state(false);
  let updatingModuleLink = $state(false);
  let removingModuleLink = $state(false);
  let assigningIssue = $state(false);
  let removingIssue = $state(false);

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

  async function onUpdateModule(
    moduleId: ProjectModuleRecord["_id"],
    input: UpdateProjectModuleInput
  ) {
    updatingModule = true;

    try {
      await updateProjectModule({
        ...input,
        moduleId,
      });
    } finally {
      updatingModule = false;
    }
  }

  async function onArchiveModule(moduleId: ProjectModuleRecord["_id"]) {
    archivingModule = true;

    try {
      await archiveProjectModule({ moduleId });
      const viewerData = deps.getViewerData();
      const activeProject = deps.getActiveProject();

      if (viewerData?.activeWorkspace && activeProject) {
        await goto(
          projectModuleHref({
            module: "modules",
            projectId: activeProject._id,
            workspaceSlug: viewerData.activeWorkspace.slug,
          })
        );
      }
    } finally {
      archivingModule = false;
    }
  }

  async function onRestoreModule(moduleId: ProjectModuleRecord["_id"]) {
    restoringModule = true;

    try {
      await restoreProjectModule({ moduleId });
      const viewerData = deps.getViewerData();
      const activeProject = deps.getActiveProject();

      if (viewerData?.activeWorkspace && activeProject) {
        await goto(
          projectModuleHref({
            module: "modules",
            projectId: activeProject._id,
            workspaceSlug: viewerData.activeWorkspace.slug,
          })
        );
      }
    } finally {
      restoringModule = false;
    }
  }

  async function onDeleteModule(moduleId: ProjectModuleRecord["_id"]) {
    deletingModule = true;

    try {
      await removeProjectModule({ moduleId });
      const viewerData = deps.getViewerData();
      const activeProject = deps.getActiveProject();

      if (viewerData?.activeWorkspace && activeProject) {
        await goto(
          projectModuleHref({
            module: "modules",
            projectId: activeProject._id,
            workspaceSlug: viewerData.activeWorkspace.slug,
          })
        );
      }
    } finally {
      deletingModule = false;
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
    assigningIssue = true;

    try {
      await addIssueAssignment({ issueId, moduleId });
    } finally {
      assigningIssue = false;
    }
  }

  async function onRemoveIssueFromModule(
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) {
    removingIssue = true;

    try {
      await removeIssueAssignment({ issueId, moduleId });
    } finally {
      removingIssue = false;
    }
  }

  async function onCreateModuleIssue(
    moduleId: ProjectModuleRecord["_id"],
    input: CreateIssueInput
  ) {
    const activeProject = deps.getActiveProject();

    if (!activeProject) {
      return;
    }

    creatingModuleIssue = true;

    try {
      await createIssue({
        ...input,
        moduleId,
        projectId: activeProject._id,
      });
    } finally {
      creatingModuleIssue = false;
    }
  }

  async function onAddModuleLink(
    moduleId: ProjectModuleRecord["_id"],
    input: AddModuleLinkInput
  ) {
    addingModuleLink = true;

    try {
      await addModuleLink({ ...input, moduleId });
    } finally {
      addingModuleLink = false;
    }
  }

  async function onUpdateModuleLink(
    linkId: ModuleLink["_id"],
    input: UpdateModuleLinkInput
  ) {
    updatingModuleLink = true;

    try {
      await updateModuleLinkMutation({ ...input, linkId });
    } finally {
      updatingModuleLink = false;
    }
  }

  async function onRemoveModuleLink(linkId: ModuleLink["_id"]) {
    removingModuleLink = true;

    try {
      await removeModuleLinkMutation({ linkId });
    } finally {
      removingModuleLink = false;
    }
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
    get creatingModuleIssue() {
      return creatingModuleIssue;
    },
    get creatingPage() {
      return creatingPage;
    },
    get creatingSprint() {
      return creatingSprint;
    },
    get updatingModule() {
      return updatingModule;
    },
    get archivingModule() {
      return archivingModule;
    },
    get restoringModule() {
      return restoringModule;
    },
    get deletingModule() {
      return deletingModule;
    },
    get addingModuleLink() {
      return addingModuleLink;
    },
    get updatingModuleLink() {
      return updatingModuleLink;
    },
    get removingModuleLink() {
      return removingModuleLink;
    },
    get assigningIssue() {
      return assigningIssue;
    },
    get removingIssue() {
      return removingIssue;
    },
    onAcceptIntakeIssue,
    onAddModuleLink,
    onArchiveModule,
    onAssignIssueToModule,
    onAssignIssueToSprint,
    onCreateIntakeIssue,
    onCreateModule,
    onCreateModuleIssue,
    onCreatePage,
    onCreateSprint,
    onDeclineIntakeIssue,
    onDeleteModule,
    onRemoveIssueFromModule,
    onRemoveModuleLink,
    onRestoreModule,
    onUpdateModule,
    onUpdateModuleLink,
    onUpdatePage,
  };
}
