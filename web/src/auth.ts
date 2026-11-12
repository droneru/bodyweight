/** The signed-in user, as returned by `GET /api/me`. */
export interface User {
  login: string;
  display_name: string;
  /** Absent if the user has no avatar of their own. */
  avatar_url: string | null;
}

/** Navigating here starts sign-in with Yandex ID. */
export const YANDEX_SIGN_IN_URL = "/api/auth/yandex/login";

/** The signed-in user, or null if nobody is signed in. */
export async function fetchMe(): Promise<User | null> {
  const response = await fetch("/api/me");
  if (response.status === 401) return null;
  if (!response.ok) throw new Error(`GET /api/me: ${response.status}`);
  return (await response.json()) as User;
}

export async function signOut(): Promise<void> {
  const response = await fetch("/api/auth/logout", { method: "POST" });
  if (!response.ok) throw new Error(`POST /api/auth/logout: ${response.status}`);
}

/** Only works when the server runs with BODYWEIGHT_DEV_LOGIN=1. */
export async function devSignIn(login: string): Promise<User> {
  const response = await fetch("/api/auth/dev-login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ login }),
  });
  if (!response.ok) throw new Error(`POST /api/auth/dev-login: ${response.status}`);
  return (await response.json()) as User;
}
