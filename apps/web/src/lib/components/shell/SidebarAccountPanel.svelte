<script lang="ts">
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";
import type { WorkspaceUser } from "$lib/components/issues/types";

let {
  onBeforeAuthExit,
  user,
}: {
  onBeforeAuthExit?: () => void;
  user: WorkspaceUser;
} = $props();

let deleteAccountOpen = $state(false);
let deletingAccount = $state(false);
let deleteAccountError = $state("");
let signingOut = $state(false);

const initials = $derived(
  user.name
    ?.trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toUpperCase() ?? user.email.slice(0, 2).toUpperCase()
);

async function signOut() {
  signingOut = true;

  try {
    onBeforeAuthExit?.();
    await authClient.signOut();
    await goto("/login");
  } finally {
    signingOut = false;
  }
}

async function deleteAccount() {
  deleteAccountError = "";
  deletingAccount = true;

  try {
    onBeforeAuthExit?.();
    const result = await authClient.deleteUser({
      callbackURL: "/login",
    });

    if (result.error) {
      deleteAccountError =
        result.error.message ?? "Could not delete your account.";
      return;
    }

    deleteAccountOpen = false;
    await goto("/login");
  } catch (error) {
    deleteAccountError =
      error instanceof Error ? error.message : "Could not delete your account.";
  } finally {
    deletingAccount = false;
  }
}
</script>

<div class="border-t border-white/10 p-3">
	<div class="mb-3 flex min-w-0 items-center gap-2 px-1">
		<div
			class="grid size-7 shrink-0 place-items-center rounded-full bg-amber-400 text-[10px] font-bold text-black"
		>
			{initials}
		</div>
		<div class="min-w-0">
			<p class="truncate text-[12px] font-medium text-zinc-300">
				{user.name || "Jet Black user"}
			</p>
			<p class="truncate text-[10px] text-zinc-600">{user.email}</p>
		</div>
	</div>
	<button
		class="flex h-8 w-full items-center rounded-md px-3 text-left text-[12px] text-zinc-500 transition hover:bg-white/5 hover:text-zinc-300 disabled:opacity-50"
		disabled={signingOut || deletingAccount}
		onclick={signOut}
		type="button"
	>
		{signingOut ? "Signing out…" : "Sign out"}
	</button>
	<button
		class="mt-1 flex h-8 w-full items-center rounded-md px-3 text-left text-[12px] text-red-400/80 transition hover:bg-red-500/10 hover:text-red-300 disabled:opacity-50"
		disabled={signingOut || deletingAccount}
		onclick={() => {
			deleteAccountError = "";
			deleteAccountOpen = true;
		}}
		type="button"
	>
		Delete account
	</button>
	{#if deleteAccountOpen}
		<div class="mt-2 rounded-md border border-red-400/20 bg-red-500/10 p-3">
			<p class="text-xs leading-5 text-red-100">
				Delete your account and workspace data? This cannot be undone.
			</p>
			<div class="mt-3 flex gap-2">
				<button
					class="h-8 rounded-md bg-red-400 px-3 text-xs font-semibold text-black transition hover:bg-red-300 disabled:opacity-50"
					disabled={deletingAccount}
					onclick={deleteAccount}
					type="button"
				>
					{deletingAccount ? "Deleting…" : "Confirm delete account"}
				</button>
				<button
					class="h-8 rounded-md border border-white/10 px-3 text-xs text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200 disabled:opacity-50"
					disabled={deletingAccount}
					onclick={() => {
						deleteAccountError = "";
						deleteAccountOpen = false;
					}}
					type="button"
				>
					Cancel
				</button>
			</div>
		</div>
	{/if}
	{#if deleteAccountError}
		<p class="mt-2 px-3 text-[11px] text-red-300" role="alert">
			{deleteAccountError}
		</p>
	{/if}
</div>
