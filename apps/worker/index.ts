import { api } from "@workspace/convex/api";
import { describeServiceStatus } from "@workspace/shared";
import { ConvexHttpClient } from "convex/browser";
import { Pool } from "pg";

const DEFAULT_PORT = 4000;
const SAMPLE_QUERY = `
  select id, name, tier, active
  from sample_customers
  order by id asc
`;

export type SampleCustomer = {
  id: number;
  name: string;
  tier: string;
  active: boolean;
};

export type HealthPayload = {
  appName: string;
  convex: {
    configured: boolean;
    connected: boolean;
    recentMessages: number;
  };
  environment: string;
  healthy: boolean;
  database: {
    configured: boolean;
    connected: boolean;
    seededCustomers: number;
  };
};

export function getServicePort(value = process.env.PORT) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : DEFAULT_PORT;
}

export function buildHealthPayload({
  convexConfigured,
  convexConnected,
  recentMessages,
  environment,
  databaseConfigured,
  databaseConnected,
  seededCustomers,
}: {
  convexConfigured: boolean;
  convexConnected: boolean;
  recentMessages: number;
  environment: string;
  databaseConfigured: boolean;
  databaseConnected: boolean;
  seededCustomers: number;
}): HealthPayload {
  return {
    appName: "worker",
    convex: {
      configured: convexConfigured,
      connected: convexConnected,
      recentMessages,
    },
    environment,
    healthy:
      (!databaseConfigured || databaseConnected) &&
      (!convexConfigured || convexConnected),
    database: {
      configured: databaseConfigured,
      connected: databaseConnected,
      seededCustomers,
    },
  };
}

export async function fetchConvexSummary(deploymentUrl?: string) {
  if (!deploymentUrl) {
    return {
      connected: false,
      recentMessages: 0,
    };
  }

  const client = new ConvexHttpClient(deploymentUrl);
  const messages = await client.query(api.messages.list, {});

  return {
    connected: true,
    recentMessages: messages.length,
  };
}

export async function fetchDatabaseSummary(connectionString?: string) {
  if (!connectionString) {
    return {
      connected: false,
      customers: [] as SampleCustomer[],
      seededCustomers: 0,
    };
  }

  const pool = new Pool({ connectionString });

  try {
    const result = await pool.query<SampleCustomer>(SAMPLE_QUERY);
    return {
      connected: true,
      customers: result.rows,
      seededCustomers: result.rowCount ?? result.rows.length,
    };
  } catch (error) {
    if (isMissingSampleTableError(error)) {
      return {
        connected: true,
        customers: [] as SampleCustomer[],
        seededCustomers: 0,
      };
    }

    throw error;
  } finally {
    await pool.end();
  }
}

function isMissingSampleTableError(error: unknown) {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    error.code === "42P01"
  );
}

export async function buildRootMessage() {
  const environment = process.env.NODE_ENV ?? "development";
  const [databaseSummary, convexSummary] = await Promise.all([
    fetchDatabaseSummary(process.env.DATABASE_URL),
    fetchConvexSummary(process.env.CONVEX_URL).catch(() => ({
      connected: false,
      recentMessages: 0,
    })),
  ]);

  return describeServiceStatus({
    appName: "worker",
    environment,
    healthy:
      (!process.env.DATABASE_URL || databaseSummary.connected) &&
      (!process.env.CONVEX_URL || convexSummary.connected),
  });
}

export function createServer() {
  return Bun.serve({
    port: getServicePort(),
    async fetch(request) {
      const url = new URL(request.url);
      const environment = process.env.NODE_ENV ?? "development";
      const hasDatabase = Boolean(process.env.DATABASE_URL);
      const hasConvex = Boolean(process.env.CONVEX_URL);

      if (url.pathname === "/health") {
        const [databaseSummary, convexSummary] = await Promise.all([
          fetchDatabaseSummary(process.env.DATABASE_URL),
          fetchConvexSummary(process.env.CONVEX_URL).catch(() => ({
            connected: false,
            recentMessages: 0,
          })),
        ]);
        const payload = buildHealthPayload({
          convexConfigured: hasConvex,
          convexConnected: convexSummary.connected,
          recentMessages: convexSummary.recentMessages,
          environment,
          databaseConfigured: hasDatabase,
          databaseConnected: databaseSummary.connected,
          seededCustomers: databaseSummary.seededCustomers,
        });

        return Response.json(payload, {
          status: payload.healthy ? 200 : 503,
        });
      }

      if (url.pathname === "/customers") {
        const summary = await fetchDatabaseSummary(process.env.DATABASE_URL);
        return Response.json({
          customers: summary.customers,
        });
      }

      if (url.pathname === "/convex") {
        const summary = await fetchConvexSummary(process.env.CONVEX_URL);
        return Response.json(summary);
      }

      return new Response(await buildRootMessage(), {
        headers: {
          "content-type": "text/plain; charset=utf-8",
        },
      });
    },
  });
}

if (import.meta.main) {
  const server = createServer();
  console.log(`worker listening on http://0.0.0.0:${server.port}`);
}
