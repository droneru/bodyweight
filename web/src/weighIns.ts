import { apiFetch } from "./api";

/** The user's bodyweight on a date, as returned by `GET /api/weigh-ins`. */
export interface WeighIn {
  /** `YYYY-MM-DD`. */
  date: string;
  /** Kilograms with one decimal, e.g. `82.5`. */
  bodyweight: string;
}

/** How far past today the server accepts a weigh-in. */
export const MAX_DAYS_AHEAD = 2;

/** All weigh-ins of the signed-in user, newest first. */
export async function fetchWeighIns(): Promise<WeighIn[]> {
  const response = await apiFetch("GET", "/api/weigh-ins");
  return (await response.json()) as WeighIn[];
}

/** Saves the weigh-in, replacing an existing one on the same date. */
export async function putWeighIn(date: string, bodyweight: string): Promise<void> {
  await apiFetch("PUT", `/api/weigh-ins/${encodeURIComponent(date)}`, { bodyweight });
}

/**
 * Turns what the user typed into what the server parses: decimal keyboards in many
 * locales give a comma, the server only takes a point. Validation is left to the server.
 */
export function normalizeBodyweight(input: string): string {
  return input.trim().replace(",", ".");
}
