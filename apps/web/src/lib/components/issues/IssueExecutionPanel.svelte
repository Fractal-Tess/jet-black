<script lang="ts">
import type { TicketRef } from "@workspace/shared/protocol";
import { defaultExecutionClient } from "$lib/execution-client/disabled";
import type { ExecutionClient } from "$lib/execution-client/types";

let {
  client = defaultExecutionClient,
  ticket,
}: {
  client?: ExecutionClient;
  ticket: Readonly<TicketRef>;
} = $props();
</script>

{#if client.mode !== "disabled"}
  <section
    aria-label="Local execution"
    class="border-b border-border px-6 py-4"
  >
    <p class="text-sm font-medium">Local execution</p>
    <p class="text-xs text-muted-foreground">
      Ready for {ticket.identifier ?? ticket.ticket_id}
    </p>
  </section>
{/if}
