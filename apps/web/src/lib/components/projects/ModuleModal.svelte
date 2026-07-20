<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@workspace/ui/components/dialog";
import { Input } from "@workspace/ui/components/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import { Textarea } from "@workspace/ui/components/textarea";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import CircleUser from "lucide-svelte/icons/circle-user";
import Users from "lucide-svelte/icons/users";
import type {
  Project,
  ProjectModuleRecord,
  WorkspaceMember,
} from "$lib/components/issues/types";
import {
  MODULE_STATUSES,
  moduleStatusDotClass,
  moduleStatusLabel,
} from "./module-status";

let {
  isOpen = false,
  members = [],
  project,
  saving = false,
  value,
  onClose,
  onSave,
}: {
  isOpen?: boolean;
  members?: WorkspaceMember[];
  project: Project;
  saving?: boolean;
  /** When provided the modal edits an existing module, otherwise it creates one. */
  value?: {
    description?: string;
    leadUserId?: string;
    memberIds?: string[];
    name: string;
    startDate?: string;
    status: ProjectModuleRecord["status"];
    targetDate?: string;
  } | null;
  onClose: () => void;
  onSave: (input: {
    description?: string | null;
    leadUserId?: string | null;
    memberIds?: string[];
    name: string;
    startDate?: string | null;
    status: ProjectModuleRecord["status"];
    targetDate?: string | null;
  }) => Promise<void>;
} = $props();

const NO_LEAD = "no-lead";

const editing = $derived(Boolean(value));

let name = $state("");
let description = $state("");
let status = $state<ProjectModuleRecord["status"]>("backlog");
let startDate = $state("");
let targetDate = $state("");
let leadUserId = $state<string>(NO_LEAD);
let memberIds = $state<string[]>([]);
let error = $state("");

$effect(() => {
  if (isOpen) {
    name = value?.name ?? "";
    description = value?.description ?? "";
    // Plane defaults new modules to "backlog" (MODULE_STATUS[0]).
    status = value?.status ?? "backlog";
    startDate = value?.startDate ?? "";
    targetDate = value?.targetDate ?? "";
    leadUserId = value?.leadUserId ?? NO_LEAD;
    memberIds = value?.memberIds ?? [];
    error = "";
  }
});

const selectedLead = $derived(
  members.find((member) => member.id === leadUserId) ?? null
);
const memberSummary = $derived.by(() => {
  if (memberIds.length === 0) {
    return "Members";
  }
  if (memberIds.length === 1) {
    const member = members.find((m) => m.id === memberIds[0]);
    return member ? (member.name ?? member.email) : "1 member";
  }
  return `${memberIds.length} members`;
});

async function submit() {
  error = "";
  const trimmed = name.trim();

  if (!trimmed) {
    error = "Module name is required";
    return;
  }

  if (startDate && targetDate && startDate > targetDate) {
    error = "Start date must not be after target date";
    return;
  }

  // When editing, cleared fields must be sent as null so the backend unsets
  // them; when creating, they are simply omitted.
  const emptyValue = editing ? null : undefined;

  try {
    await onSave({
      description: description.trim() || emptyValue,
      leadUserId: leadUserId === NO_LEAD ? emptyValue : leadUserId,
      memberIds,
      name: trimmed,
      startDate: startDate || emptyValue,
      status,
      targetDate: targetDate || emptyValue,
    });
  } catch (cause) {
    error =
      cause instanceof Error ? cause.message : "Could not save the module.";
  }
}

const pillClass =
  "flex h-7 w-auto cursor-pointer items-center gap-1.5 rounded-lg border border-input bg-background px-2.5 text-xs text-foreground transition-colors hover:bg-muted";
const dateInputClass =
  "w-[6.5rem] cursor-pointer bg-transparent text-xs text-foreground outline-none placeholder:text-muted-foreground";
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
        <span class="text-foreground">
          {editing ? "Update module" : "Create module"}
        </span>
      </DialogTitle>
    </DialogHeader>

    <form
      onsubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <div class="px-5 pt-3">
        <label class="sr-only" for="module-name">Module name</label>
        <!-- svelte-ignore a11y_autofocus -->
        <Input
          autofocus
          bind:value={name}
          class="h-auto border-0 bg-transparent px-0 text-lg font-medium shadow-none placeholder:text-muted-foreground focus-visible:ring-0"
          id="module-name"
          maxlength={255}
          placeholder="Title"
          required
        />

        <label class="sr-only" for="module-description">
          Module description
        </label>
        <Textarea
          aria-label="Module description"
          bind:value={description}
          class="mt-2 min-h-20 resize-y border-0 bg-transparent px-0 shadow-none focus-visible:ring-0"
          id="module-description"
          placeholder="Description…"
        />
      </div>

      <div class="flex flex-wrap items-center gap-2 px-5 pb-4 pt-1">
        <div class={pillClass} title="Start date">
          <CalendarDays class="size-3.5 text-muted-foreground" />
          <input
            aria-label="Start date"
            bind:value={startDate}
            class={dateInputClass}
            max={targetDate || undefined}
            type="date"
          />
        </div>

        <div class={pillClass} title="Target date">
          <CalendarDays class="size-3.5 text-muted-foreground" />
          <input
            aria-label="Target date"
            bind:value={targetDate}
            class={dateInputClass}
            min={startDate || undefined}
            type="date"
          />
        </div>

        <Select
          onValueChange={(next) => {
            if (next) {
              status = next as ProjectModuleRecord["status"];
            }
          }}
          type="single"
          value={status}
        >
          <SelectTrigger aria-label="Status" class={pillClass}>
            <span
              aria-hidden="true"
              class="size-2 rounded-full {moduleStatusDotClass(status)}"
            ></span>
            {moduleStatusLabel(status)}
          </SelectTrigger>
          <SelectContent>
            {#each MODULE_STATUSES as option (option.value)}
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
            onValueChange={(next) => (leadUserId = next ?? NO_LEAD)}
            type="single"
            value={leadUserId}
          >
            <SelectTrigger aria-label="Lead" class={pillClass}>
              <CircleUser class="size-3.5 text-muted-foreground" />
              {selectedLead ? (selectedLead.name ?? selectedLead.email) : "Lead"}
            </SelectTrigger>
            <SelectContent>
              <SelectItem class="text-sm" label="No lead" value={NO_LEAD} />
              {#each members as member (member.id)}
                <SelectItem
                  class="text-sm"
                  label={member.name ?? member.email}
                  value={member.id}
                />
              {/each}
            </SelectContent>
          </Select>

          <Select
            onValueChange={(next) => (memberIds = next ?? [])}
            type="multiple"
            value={memberIds}
          >
            <SelectTrigger aria-label="Members" class={pillClass}>
              <Users class="size-3.5 text-muted-foreground" />
              {memberSummary}
            </SelectTrigger>
            <SelectContent>
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
      </div>

      {#if error}
        <div class="px-5 pb-2">
          <p class="text-sm text-destructive" role="alert">{error}</p>
        </div>
      {/if}

      <DialogFooter class="mx-0 mb-0 rounded-b-xl px-5 py-3">
        <Button
          onclick={onClose}
          size="sm"
          variant="outline"
        >
          Cancel
        </Button>
        <Button
          disabled={saving || !name.trim()}
          size="sm"
          type="submit"
        >
          {#if saving}
            <span
              aria-hidden="true"
              class="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent"
            ></span>
          {/if}
          {#if saving}
            Saving…
          {:else if editing}
            Save changes
          {:else}
            Create module
          {/if}
        </Button>
      </DialogFooter>
    </form>
  </DialogContent>
</Dialog>
