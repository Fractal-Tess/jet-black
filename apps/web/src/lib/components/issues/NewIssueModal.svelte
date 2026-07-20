<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Checkbox } from "@workspace/ui/components/checkbox";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@workspace/ui/components/dialog";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import CircleUser from "lucide-svelte/icons/circle-user";
import { tick } from "svelte";
import DatePicker from "./DatePicker.svelte";
import DescriptionEditor from "./DescriptionEditor.svelte";
import { useDescriptionUploader } from "./description-upload";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type {
  IssuePriority,
  IssueState,
  Project,
  WorkspaceMember,
} from "./types";

let {
  creating = false,
  initialStateId,
  isOpen = false,
  members = [],
  onClose,
  onCreate,
  project,
  states = [],
}: {
  creating?: boolean;
  initialStateId?: IssueState["_id"];
  isOpen?: boolean;
  members?: WorkspaceMember[];
  onClose: () => void;
  onCreate: (
    input: {
      assigneeUserId?: string;
      description?: string;
      priority: IssuePriority;
      startDate?: string;
      stateId?: IssueState["_id"];
      targetDate?: string;
      title: string;
    },
    options?: { navigate?: boolean }
  ) => Promise<void>;
  project: Project;
  states?: IssueState[];
} = $props();

const PRIORITY_OPTIONS: { label: string; value: IssuePriority }[] = [
  { label: "None", value: "none" },
  { label: "Low", value: "low" },
  { label: "Medium", value: "medium" },
  { label: "High", value: "high" },
  { label: "Urgent", value: "urgent" },
];

const UNASSIGNED = "unassigned";

let title = $state("");
let description = $state("");
let priority = $state<IssuePriority>("medium");
// The modal mounts fresh on every open, so initializing from the prop is
// enough to prefill the state (e.g. the kanban column the + was clicked in).
// svelte-ignore state_referenced_locally
let stateId = $state<string | undefined>(initialStateId);
let assigneeUserId = $state<string>(UNASSIGNED);
let startDate = $state<string | null>(null);
let targetDate = $state<string | null>(null);
let createMore = $state(false);
let error = $state("");
let titleInput = $state<HTMLInputElement | null>(null);
let descriptionEditor = $state<DescriptionEditor | null>(null);

const uploadDescriptionFile = useDescriptionUploader(() => project.workspaceId);

const priorityLabel = $derived(
  PRIORITY_OPTIONS.find((option) => option.value === priority)?.label ?? "None"
);
const selectedState = $derived(
  states.find((state) => state._id === stateId) ?? states[0]
);
const selectedAssignee = $derived(
  members.find((member) => member.id === assigneeUserId) ?? null
);

function reset() {
  title = "";
  description = "";
  descriptionEditor?.setMarkdown("");
  priority = "medium";
  stateId = undefined;
  assigneeUserId = UNASSIGNED;
  startDate = null;
  targetDate = null;
  error = "";
}

async function submit() {
  error = "";

  if (!title.trim()) {
    error = "Title is required.";
    return;
  }

  try {
    await onCreate(
      {
        assigneeUserId:
          assigneeUserId === UNASSIGNED ? undefined : assigneeUserId,
        description: description.trim() || undefined,
        priority,
        startDate: startDate ?? undefined,
        stateId: selectedState?._id,
        targetDate: targetDate ?? undefined,
        title,
      },
      { navigate: !createMore }
    );

    if (createMore) {
      // Plane keeps the property selections and only clears the content
      // fields so several similar items can be created in a row.
      title = "";
      description = "";
      descriptionEditor?.setMarkdown("");
      error = "";
      await tick();
      titleInput?.focus();
      return;
    }

    reset();
    onClose();
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Could not create the work item.";
  }
}

const pillClass =
  "flex h-7 w-auto cursor-pointer items-center gap-1.5 rounded-lg border border-input bg-background px-2.5 text-xs text-foreground transition-colors hover:bg-muted";
</script>

<Dialog
  onOpenChange={(open) => {
    if (!open) {
      onClose();
    }
  }}
  open={isOpen}
>
  <DialogContent
    class="top-[15vh] max-w-xl translate-y-0 gap-0 p-0 sm:max-w-xl"
  >
    <DialogHeader class="px-5 pt-4">
      <DialogTitle
        class="flex items-center gap-1.5 text-left text-xs font-normal text-muted-foreground"
      >
        <span
          class="grid size-4 place-items-center rounded bg-muted font-mono text-meta uppercase text-muted-foreground"
        >
          {project.key.slice(0, 2)}
        </span>
        {project.name}
        <span class="text-muted-foreground">›</span>
        <span class="text-foreground">Create work item</span>
      </DialogTitle>
    </DialogHeader>

    <form
      onsubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <div class="px-5 pt-3">
        <label class="sr-only" for="issue-title">Title</label>
        <!-- svelte-ignore a11y_autofocus -->
        <Input
          autofocus
          bind:ref={titleInput}
          bind:value={title}
          class="h-auto border-0 bg-transparent px-0 text-lg font-medium shadow-none placeholder:text-muted-foreground focus-visible:ring-0"
          id="issue-title"
          placeholder="What needs to be done?"
          required
        />

        <DescriptionEditor
          ariaLabel="Issue description"
          bind:this={descriptionEditor}
          class="mt-2 min-h-20 cursor-text"
          onUpdate={(markdown) => (description = markdown)}
          placeholder="Add a description…"
          uploadFile={uploadDescriptionFile}
        />
      </div>

      <div class="flex flex-wrap items-center gap-2 px-5 pb-4 pt-1">
        {#if states.length > 0}
          <Select
            onValueChange={(value) => (stateId = value)}
            type="single"
            value={selectedState?._id}
          >
            <SelectTrigger aria-label="State" class={pillClass}>
              {#if selectedState}
                <StateTypeIcon
                  class="size-3.5"
                  type={selectedState.type}
                />
                {selectedState.name}
              {:else}
                State
              {/if}
            </SelectTrigger>
            <SelectContent>
              {#each states as state (state._id)}
                <SelectItem
                  class="text-sm"
                  label={state.name}
                  value={state._id}
                />
              {/each}
            </SelectContent>
          </Select>
        {/if}

        <Select
          onValueChange={(value) => (priority = value as IssuePriority)}
          type="single"
          value={priority}
        >
          <SelectTrigger aria-label="Priority" class={pillClass}>
            <PriorityIcon class="size-3.5" {priority} />
            {priorityLabel}
          </SelectTrigger>
          <SelectContent>
            {#each PRIORITY_OPTIONS as option (option.value)}
              <SelectItem
                class="text-sm"
                label={option.label}
                value={option.value}
              />
            {/each}
          </SelectContent>
        </Select>

        {#if members.length > 0}
          <Select
            onValueChange={(value) => (assigneeUserId = value ?? UNASSIGNED)}
            type="single"
            value={assigneeUserId}
          >
            <SelectTrigger aria-label="Assignee" class={pillClass}>
              <CircleUser class="size-3.5 text-muted-foreground" />
              {selectedAssignee
                ? (selectedAssignee.name ?? selectedAssignee.email)
                : "Assignee"}
            </SelectTrigger>
            <SelectContent>
              <SelectItem class="text-sm" label="Unassigned" value={UNASSIGNED} />
              {#each members as member (member.id)}
                <SelectItem
                  class="text-sm"
                  label={member.name ?? member.email}
                  value={member.id}
                />
              {/each}
            </SelectContent>
          </Select>
        {/if}

        <div class={pillClass} title="Start date">
          <CalendarDays class="size-3.5 text-muted-foreground" />
          <DatePicker
            disabledHint="The start date must be before the due date"
            maxDate={targetDate}
            onChange={(date) => (startDate = date)}
            placeholder="Start date"
            value={startDate}
          />
        </div>

        <div class={pillClass} title="Due date">
          <CalendarDays class="size-3.5 text-muted-foreground" />
          <DatePicker
            disabledHint="The due date must be after the start date"
            minDate={startDate}
            onChange={(date) => (targetDate = date)}
            placeholder="Due date"
            value={targetDate}
          />
        </div>
      </div>

      {#if error}
        <div class="px-5 pb-2">
          <p class="text-sm text-destructive" role="alert">{error}</p>
        </div>
      {/if}

      <DialogFooter class="mx-0 mb-0 flex-row items-center justify-between rounded-b-xl px-5 py-3">
        <div class="flex items-center gap-2">
          <Checkbox
            bind:checked={createMore}
            class="size-3.5"
            id="issue-create-more"
          />
          <Label
            class="cursor-pointer text-xs font-normal text-muted-foreground"
            for="issue-create-more"
          >
            Create more
          </Label>
        </div>

        <div class="flex items-center gap-3">
          <Button
            onclick={onClose}
            size="sm"
            variant="outline"
          >
            Discard
          </Button>
          <Button
            disabled={creating || !title.trim()}
            size="sm"
            type="submit"
          >
            {#if creating}
              <span
                aria-hidden="true"
                class="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent"
              ></span>
            {/if}
            {creating ? "Creating…" : "Create work item"}
          </Button>
        </div>
      </DialogFooter>
    </form>
  </DialogContent>
</Dialog>
