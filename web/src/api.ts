/** The session is missing or expired: the user has to sign in again. */
export class UnauthorizedError extends Error {
  constructor() {
    super("not signed in");
  }
}

/** The server refused the request. For a 400 or a 409 the message is meant for the user. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

/** Calls the API as the signed-in user; `body` is sent as JSON. */
export async function apiFetch(method: string, path: string, body?: unknown): Promise<Response> {
  const response = await fetch(path, {
    method,
    headers: body === undefined ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (response.status === 401) throw new UnauthorizedError();
  if (!response.ok) {
    const forUser = response.status === 400 || response.status === 409;
    const text = forUser ? await response.text() : "";
    throw new ApiError(response.status, text || `${method} ${path}: ${response.status}`);
  }
  return response;
}

/** What to tell the user about a failed request: the server's message if it has one for them. */
export function errorMessage(e: unknown, fallback: string): string {
  const forUser = e instanceof ApiError && (e.status === 400 || e.status === 409);
  return forUser ? e.message.charAt(0).toUpperCase() + e.message.slice(1) : fallback;
}
