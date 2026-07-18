<script lang="ts">
import { setupAuth, setupConvex } from "convex-svelte";
import { onMount, untrack } from "svelte";
import "../app.css";
import { authClient } from "$lib/auth-client";

let { children, data } = $props();

// Destructure SSR data first so it's available for initializing snapshot.
const { convexUrl, isAuthenticated: initiallyAuthenticated } = untrack(
  () => data
);

// Reactive snapshot for Better Auth session state.
// Seeded from SSR data — no eager fetch during server render.
let authSnapshot = $state({
  isAuthenticated: initiallyAuthenticated,
  isLoading: false,
});

if (convexUrl) {
  setupConvex(convexUrl);
  setupAuth(
    () => ({
      fetchAccessToken: async () => {
        const result = await authClient.convex.token({
          fetchOptions: { throw: false },
        });

        return result.data?.token ?? null;
      },
      isAuthenticated: authSnapshot.isAuthenticated,
      isLoading: authSnapshot.isLoading,
    }),
    {
      initialState: {
        isAuthenticated: initiallyAuthenticated,
      },
    }
  );
}

onMount(() => {
  const session = authClient.useSession();

  const unsub = session.subscribe((value) => {
    authSnapshot = {
      isAuthenticated: Boolean(value.data?.session),
      isLoading: value.isPending,
    };
  });

  return unsub;
});
</script>

<svelte:head>
  <title>Jet Black</title>
  <meta
    name="description"
    content="A real-time workspace for projects, issues, and code review."
  />
</svelte:head>

{@render children()}
