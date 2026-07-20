<script lang="ts">
import NewIssueModal from "$lib/components/issues/NewIssueModal.svelte";
import type {
  CreateIssueInput,
  IssueState,
  Project,
  WorkspaceMember,
} from "$lib/components/issues/types";

let {
  open,
  activeProject,
  creating,
  initialStateId,
  members = [],
  onClose,
  onCreate,
  states = [],
}: {
  open: boolean;
  activeProject: Project | null;
  creating: boolean;
  initialStateId?: IssueState["_id"];
  members?: WorkspaceMember[];
  onClose: () => void;
  onCreate: (
    input: CreateIssueInput,
    options?: { navigate?: boolean }
  ) => Promise<void>;
  states?: IssueState[];
} = $props();
</script>

{#if open && activeProject}
  <NewIssueModal
    {creating}
    {initialStateId}
    isOpen={open}
    {members}
    {onClose}
    {onCreate}
    project={activeProject}
    {states}
  />
{/if}
