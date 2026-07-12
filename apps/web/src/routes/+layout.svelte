<script lang="ts">
import { setupAuth, setupConvex } from "convex-svelte";
import { untrack } from "svelte";
import "../app.css";
import { authClient } from "$lib/auth-client";

let { children, data } = $props();

const session = authClient.useSession();
const { convexUrl, isAuthenticated: initiallyAuthenticated } = untrack(
  () => data
);

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
      isAuthenticated: Boolean($session.data?.session),
      isLoading: $session.isPending,
    }),
    {
      initialState: {
        isAuthenticated: initiallyAuthenticated,
      },
    }
  );
}
</script>

<svelte:head>
  <title>Jet Black</title>
  <meta
    name="description"
    content="A real-time workspace for projects, issues, and code review."
  />
</svelte:head>

{@render children()}
