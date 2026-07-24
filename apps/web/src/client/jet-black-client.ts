import {
  type LocalCommand,
  type LocalCommandResponse,
  PROTOCOL_VERSION,
  type ProductClientMessage,
  type ProductCommand,
  type ProductCommandResponse,
  type ProductEventPage,
  type ProductServerMessage,
  type ProductSnapshot,
  type ProductUser,
  type ResponseEnvelope,
} from "@workspace/shared/protocol";

const INSTANCE_STORAGE_KEY = "jet-black.instance";
const CSRF_STORAGE_KEY = "jet-black.csrf";
const CONNECTIONS_STORAGE_KEY = "jet-black.connections";
const TRAILING_SLASHES = /\/+$/;

type SessionResponse = {
  csrf_token: string;
  expires_at_ms: number;
  user: ProductUser;
};

export type ManagedUser = {
  instance_admin: boolean;
  pending: boolean;
  user: ProductUser;
};

export type RemoteConnection = {
  baseUrl: string;
  csrfToken: string;
  expiresAtMs: number;
  id: string;
  name: string;
  sessionToken: string;
};

type PairingExchangeResponse = {
  csrf_token: string;
  expires_at_ms: number;
  session_token: string;
  user: ProductUser;
};

export class JetBlackClientError extends Error {
  readonly code: string;
  readonly status: number;

  constructor(code: string, message: string, status: number) {
    super(message);
    this.name = "JetBlackClientError";
    this.code = code;
    this.status = status;
  }
}

export class JetBlackClient {
  readonly baseUrl: string;
  #csrfToken: string | null = null;
  #sessionToken: string | null = null;

  constructor(
    baseUrl = configuredInstance(),
    credentials?: { csrfToken: string; sessionToken: string }
  ) {
    this.baseUrl = normalizeBaseUrl(baseUrl);
    this.#csrfToken =
      credentials?.csrfToken ??
      sessionStorage.getItem(`${CSRF_STORAGE_KEY}:${this.baseUrl || "local"}`);
    this.#sessionToken = credentials?.sessionToken ?? null;
  }

  get csrfToken(): string | null {
    return this.#csrfToken;
  }

  async bootstrap(): Promise<{
    authentication: string;
    profile: string;
    protocol_version: string;
    websocket_path: string;
  }> {
    return await this.#request("/api/bootstrap");
  }

  async currentSession(): Promise<ProductUser> {
    return await this.#request("/api/auth/session");
  }

  async localLogin(): Promise<SessionResponse> {
    const session = await this.#request<SessionResponse>("/api/auth/local", {
      method: "POST",
    });
    this.#csrfToken = session.csrf_token;
    this.#saveCsrf();
    return session;
  }

  async signup(input: {
    display_name: string;
    email: string;
    password: string;
  }): Promise<{ status: string; user: ProductUser }> {
    const response = await this.#request<{ status: string; user: ProductUser }>(
      "/api/auth/signup",
      {
        body: JSON.stringify(input),
        headers: { "content-type": "application/json" },
        method: "POST",
      }
    );
    return response;
  }

  async issueDesktopToken(
    deviceName: string
  ): Promise<{ expires_at_ms: number; token: string }> {
    return await this.#request("/api/desktop/pair", {
      body: JSON.stringify({ device_name: deviceName }),
      headers: {
        "content-type": "application/json",
        ...this.#mutationHeaders(),
      },
      method: "POST",
    });
  }

  async managedUsers(): Promise<ManagedUser[]> {
    return await this.#request("/api/admin/users");
  }

  async approveUser(userId: string): Promise<void> {
    await this.#request(`/api/admin/users/${userId}/approve`, {
      headers: this.#mutationHeaders(),
      method: "POST",
    });
  }

  async assignWorkspaceMember(
    workspaceId: string,
    userId: string,
    role: "admin" | "guest" | "member"
  ): Promise<void> {
    await this.#request(`/api/admin/workspaces/${workspaceId}/members`, {
      body: JSON.stringify({ role, user_id: userId }),
      headers: {
        "content-type": "application/json",
        ...this.#mutationHeaders(),
      },
      method: "POST",
    });
  }

  static async connectRemote(input: {
    baseUrl: string;
    deviceName: string;
    name: string;
    token: string;
  }): Promise<RemoteConnection> {
    const baseUrl = normalizeBaseUrl(input.baseUrl);
    const response = await fetch(`${baseUrl}/api/desktop/exchange`, {
      body: JSON.stringify({ token: input.token }),
      headers: { "content-type": "application/json" },
      method: "POST",
    });
    if (!response.ok) {
      throw new JetBlackClientError(
        "pairing_failed",
        "The connection token is invalid or expired.",
        response.status
      );
    }
    const exchange = (await response.json()) as PairingExchangeResponse;
    return {
      baseUrl,
      csrfToken: exchange.csrf_token,
      expiresAtMs: exchange.expires_at_ms,
      id: crypto.randomUUID(),
      name: input.name.trim() || new URL(baseUrl).host,
      sessionToken: exchange.session_token,
    };
  }

  async login(email: string, password: string): Promise<SessionResponse> {
    const session = await this.#request<SessionResponse>("/api/auth/password", {
      body: JSON.stringify({ email, password }),
      headers: { "content-type": "application/json" },
      method: "POST",
    });
    this.#csrfToken = session.csrf_token;
    this.#saveCsrf();
    return session;
  }

  async exchangeLaunchToken(token: string): Promise<SessionResponse> {
    const session = await this.#request<SessionResponse>("/api/auth/launch", {
      body: JSON.stringify({ token }),
      headers: { "content-type": "application/json" },
      method: "POST",
    });
    this.#csrfToken = session.csrf_token;
    this.#saveCsrf();
    return session;
  }

  async logout(): Promise<void> {
    await this.#request("/api/auth/logout", {
      method: "POST",
      headers: this.#mutationHeaders(),
    });
    this.#csrfToken = null;
    this.#saveCsrf();
  }

  async snapshot(workspaceId?: string): Promise<ProductSnapshot> {
    const query = workspaceId
      ? `?workspace_id=${encodeURIComponent(workspaceId)}`
      : "";
    return await this.#request(`/api/product/snapshot${query}`);
  }

  async command(command: ProductCommand): Promise<ProductCommandResponse> {
    const envelope = {
      version: PROTOCOL_VERSION,
      request_id: crypto.randomUUID(),
      payload: command,
    };
    const response = await this.#request<
      ResponseEnvelope<ProductCommandResponse>
    >("/api/product/commands", {
      body: JSON.stringify(envelope),
      headers: {
        "content-type": "application/json",
        ...this.#mutationHeaders(),
      },
      method: "POST",
    });
    if (response.result.status === "error") {
      throw new JetBlackClientError(
        response.result.data.code,
        response.result.data.message,
        400
      );
    }
    return response.result.data;
  }

  async executionBootstrap(): Promise<{
    default_provider: { kind: string; model: string | null };
    enabled_features: string[];
    profile: string;
    protocol_version: string;
    provider_availability: string[];
    version: string;
  }> {
    return await this.#request("/api/execution/bootstrap");
  }

  async executionCommand(command: LocalCommand): Promise<LocalCommandResponse> {
    const envelope = {
      version: PROTOCOL_VERSION,
      request_id: crypto.randomUUID(),
      payload: command,
    };
    const response = await this.#request<
      ResponseEnvelope<LocalCommandResponse>
    >("/api/execution/commands", {
      body: JSON.stringify(envelope),
      headers: {
        "content-type": "application/json",
        ...this.#mutationHeaders(),
      },
      method: "POST",
    });
    if (response.result.status === "error") {
      throw new JetBlackClientError(
        response.result.data.code,
        response.result.data.message,
        400
      );
    }
    return response.result.data;
  }

  async events(
    workspaceId: string,
    afterCursor: number
  ): Promise<ProductEventPage> {
    const query = new URLSearchParams({
      after_cursor: afterCursor.toString(),
      workspace_id: workspaceId,
    });
    return await this.#request(`/api/product/events?${query}`);
  }

  connect(
    workspaceId: string,
    afterCursor: number,
    onMessage: (message: ProductServerMessage) => void,
    onConnectionChange: (connected: boolean) => void
  ): () => void {
    if (this.#sessionToken) {
      let disposed = false;
      let cursor = afterCursor;
      const poll = async () => {
        if (disposed) {
          return;
        }
        try {
          const page = await this.events(workspaceId, cursor);
          cursor = page.next_cursor;
          onConnectionChange(true);
          if (page.events.length > 0) {
            onMessage({ type: "events", data: page });
          }
        } catch {
          onConnectionChange(false);
        }
        if (!disposed) {
          window.setTimeout(poll, 2000);
        }
      };
      poll();
      return () => {
        disposed = true;
      };
    }
    let disposed = false;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;
    let socket: WebSocket | undefined;
    let retryDelay = 500;

    const open = () => {
      if (disposed) {
        return;
      }
      const url = new URL("/api/ws", this.baseUrl || window.location.origin);
      url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
      socket = new WebSocket(url);
      socket.addEventListener("open", () => {
        retryDelay = 500;
        onConnectionChange(true);
        const subscribe: ProductClientMessage = {
          type: "subscribe",
          data: { after_cursor: afterCursor, workspace_id: workspaceId },
        };
        socket?.send(JSON.stringify(subscribe));
      });
      socket.addEventListener("message", (event) => {
        if (typeof event.data !== "string") {
          return;
        }
        const message = JSON.parse(event.data) as ProductServerMessage;
        onMessage(message);
      });
      socket.addEventListener("close", () => {
        onConnectionChange(false);
        if (disposed) {
          return;
        }
        retryTimer = setTimeout(open, retryDelay);
        retryDelay = Math.min(retryDelay * 2, 10_000);
      });
    };
    open();
    return () => {
      disposed = true;
      if (retryTimer) {
        clearTimeout(retryTimer);
      }
      socket?.close();
    };
  }

  #mutationHeaders(): Record<string, string> {
    if (!this.#csrfToken) {
      throw new JetBlackClientError(
        "missing_csrf",
        "Sign in again before changing data.",
        401
      );
    }
    return { "x-csrf-token": this.#csrfToken };
  }

  #saveCsrf(): void {
    const key = `${CSRF_STORAGE_KEY}:${this.baseUrl || "local"}`;
    if (this.#csrfToken) {
      sessionStorage.setItem(key, this.#csrfToken);
      return;
    }
    sessionStorage.removeItem(key);
  }

  async #request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const headers = new Headers(init.headers);
    if (this.#sessionToken) {
      headers.set("authorization", `Bearer ${this.#sessionToken}`);
    }
    const response = await fetch(`${this.baseUrl}${path}`, {
      ...init,
      headers,
      credentials: "include",
    });
    if (!response.ok) {
      const error = (await response.json().catch(() => null)) as {
        code?: string;
        message?: string;
      } | null;
      throw new JetBlackClientError(
        error?.code ?? "request_failed",
        error?.message ?? `Request failed with status ${response.status}`,
        response.status
      );
    }
    if (response.status === 204) {
      return undefined as T;
    }
    return (await response.json()) as T;
  }
}

export function configuredInstance(): string {
  if (typeof window === "undefined") {
    return "";
  }
  return localStorage.getItem(INSTANCE_STORAGE_KEY) ?? "";
}

export function saveConfiguredInstance(value: string): string {
  const normalized = normalizeBaseUrl(value);
  localStorage.setItem(INSTANCE_STORAGE_KEY, normalized);
  return normalized;
}

export function savedConnections(): RemoteConnection[] {
  if (typeof window === "undefined") {
    return [];
  }
  try {
    const value = JSON.parse(
      localStorage.getItem(CONNECTIONS_STORAGE_KEY) ?? "[]"
    ) as RemoteConnection[];
    return value.filter(
      (connection) =>
        connection.expiresAtMs > Date.now() &&
        Boolean(connection.baseUrl && connection.sessionToken)
    );
  } catch {
    return [];
  }
}

export function saveConnections(connections: RemoteConnection[]): void {
  localStorage.setItem(CONNECTIONS_STORAGE_KEY, JSON.stringify(connections));
}

function normalizeBaseUrl(value: string): string {
  const trimmed = value.trim().replace(TRAILING_SLASHES, "");
  if (!trimmed) {
    return "";
  }
  const url = new URL(trimmed);
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new JetBlackClientError(
      "invalid_instance",
      "Instance URL must use HTTP or HTTPS.",
      0
    );
  }
  return url.origin;
}
