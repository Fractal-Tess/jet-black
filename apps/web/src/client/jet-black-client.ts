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
const TRAILING_SLASHES = /\/+$/;

type SessionResponse = {
  csrf_token: string;
  expires_at_ms: number;
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

  constructor(baseUrl = configuredInstance()) {
    this.baseUrl = normalizeBaseUrl(baseUrl);
    this.#csrfToken = sessionStorage.getItem(
      `${CSRF_STORAGE_KEY}:${this.baseUrl || "local"}`
    );
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
    const response = await fetch(`${this.baseUrl}${path}`, {
      ...init,
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
  const query = new URLSearchParams(window.location.search).get("instance");
  if (query) {
    const normalized = normalizeBaseUrl(query);
    localStorage.setItem(INSTANCE_STORAGE_KEY, normalized);
    return normalized;
  }
  return localStorage.getItem(INSTANCE_STORAGE_KEY) ?? "";
}

export function saveConfiguredInstance(value: string): string {
  const normalized = normalizeBaseUrl(value);
  localStorage.setItem(INSTANCE_STORAGE_KEY, normalized);
  return normalized;
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
