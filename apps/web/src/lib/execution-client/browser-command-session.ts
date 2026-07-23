import { z } from "zod";
import { ExecutionClientError, isStructuredError } from "./errors";
import { createHttpExecutionClient } from "./http";
import type {
  ExecutionClient,
  FetchTransport,
  RequestIdFactory,
} from "./types";

const SESSION_STORAGE_KEY = "jet-black.execution-session";
const HASH_PREFIX_PATTERN = /^#/u;

type SessionExchange = {
  csrf_token: string;
  expires_at_unix_ms: number;
};

type BrowserLocation = Pick<Location, "hash" | "pathname" | "search">;
type BrowserHistory = Pick<History, "replaceState" | "state">;
type BrowserStorage = Pick<Storage, "getItem" | "removeItem" | "setItem">;

export type BrowserCommandSessionOptions = {
  commandEndpoint?: string;
  createRequestId?: RequestIdFactory;
  exchangeEndpoint?: string;
  fetch?: FetchTransport;
  history?: BrowserHistory;
  location?: BrowserLocation;
  now?: () => number;
  storage?: BrowserStorage;
};

const sessionExchangeSchema = z.object({
  csrf_token: z.string().min(1),
  expires_at_unix_ms: z.int(),
});

const parseSessionExchange = (value: unknown): SessionExchange => {
  const result = sessionExchangeSchema.safeParse(value);
  if (!result.success) {
    throw new ExecutionClientError({
      code: "invalid_response",
      message: "Execution service returned an invalid session exchange.",
      retryable: false,
    });
  }

  return result.data;
};

const parseStoredSession = (
  value: string | null,
  now: number
): SessionExchange | null => {
  if (value === null) {
    return null;
  }

  try {
    const session = parseSessionExchange(JSON.parse(value));
    return session.expires_at_unix_ms > now ? session : null;
  } catch {
    return null;
  }
};

const removeLaunchToken = (
  location: BrowserLocation,
  history: BrowserHistory
): string | null => {
  const parameters = new URLSearchParams(
    location.hash.replace(HASH_PREFIX_PATTERN, "")
  );
  const launchToken = parameters.get("exchange");
  if (launchToken === null) {
    return null;
  }

  parameters.delete("exchange");
  const remainingFragment = parameters.toString();
  const nextUrl = `${location.pathname}${location.search}${
    remainingFragment.length > 0 ? `#${remainingFragment}` : ""
  }`;
  history.replaceState(history.state, "", nextUrl);
  return launchToken;
};

const exchangeLaunchToken = async (
  launchToken: string,
  endpoint: string,
  fetchTransport: FetchTransport
): Promise<SessionExchange> => {
  let response: Response;
  try {
    response = await fetchTransport(endpoint, {
      body: JSON.stringify({ token: launchToken }),
      credentials: "same-origin",
      headers: { "content-type": "application/json" },
      method: "POST",
    });
  } catch {
    throw new ExecutionClientError({
      code: "transport_error",
      message: "Unable to exchange the local launch token.",
      retryable: true,
    });
  }

  let body: unknown;
  try {
    body = await response.json();
  } catch {
    throw new ExecutionClientError({
      code: "invalid_response",
      message: "Execution service returned invalid session JSON.",
      retryable: false,
    });
  }

  if (!response.ok) {
    throw new ExecutionClientError(
      isStructuredError(body)
        ? body
        : {
            code: "session_exchange_failed",
            message: `Session exchange returned HTTP ${response.status}.`,
            retryable: response.status >= 500,
          }
    );
  }

  return parseSessionExchange(body);
};

const resolveSession = async ({
  exchangeEndpoint,
  fetchTransport,
  history,
  location,
  now,
  storage,
}: {
  exchangeEndpoint: string;
  fetchTransport: FetchTransport;
  history: BrowserHistory;
  location: BrowserLocation;
  now: number;
  storage: BrowserStorage;
}): Promise<SessionExchange> => {
  const launchToken = removeLaunchToken(location, history);
  if (launchToken !== null) {
    const session = await exchangeLaunchToken(
      launchToken,
      exchangeEndpoint,
      fetchTransport
    );
    storage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
    return session;
  }

  const session = parseStoredSession(storage.getItem(SESSION_STORAGE_KEY), now);
  if (session !== null) {
    return session;
  }

  storage.removeItem(SESSION_STORAGE_KEY);
  throw new ExecutionClientError({
    code: "session_required",
    message: "Open the standalone launch URL to start a local session.",
    retryable: false,
  });
};

export const createBrowserCommandSession = async (
  options: BrowserCommandSessionOptions = {}
): Promise<ExecutionClient> => {
  const fetchTransport = options.fetch ?? globalThis.fetch;
  const session = await resolveSession({
    exchangeEndpoint: options.exchangeEndpoint ?? "/api/session/exchange",
    fetchTransport,
    history: options.history ?? globalThis.history,
    location: options.location ?? globalThis.location,
    now: (options.now ?? Date.now)(),
    storage: options.storage ?? globalThis.sessionStorage,
  });

  return createHttpExecutionClient({
    createRequestId: options.createRequestId,
    csrfToken: session.csrf_token,
    endpoint: options.commandEndpoint ?? "/api/commands",
    fetch: fetchTransport,
  });
};
