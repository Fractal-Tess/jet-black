<script lang="ts">
import NewIssueModal from "$lib/components/issues/NewIssueModal.svelte";
import type { CreateIssueInput, Project } from "$lib/components/issues/types";

let {
  open,
  activeProject,
  creating,
  onClose,
  onCreate,
}: {
  open: boolean;
  activeProject: Project | null;
  creating: boolean;
  onClose: () => void;
  onCreate: (input: CreateIssueInput) => Promise<void>;
} = $props();
</script>

{#if open && activeProject}
  <NewIssueModal
    {creating}
    isOpen={open}
    {onClose}
    {onCreate}
    project={activeProject}
  />
{/if}
