/** The session is missing or expired: the user has to sign in again. */
export class UnauthorizedError extends Error {
  constructor() {
    super("not signed in");
  }
}

/** The server refused the request. For a 400 the message is meant for the user. */
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
    const text = response.status === 400 ? await response.text() : "";
    throw new ApiError(response.status, text || `${method} ${path}: ${response.status}`);
  }
  return response;
}
