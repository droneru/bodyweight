import { apiFetch } from "./api";

/** A PR: the best result in a standard exercise over a period, and the set it comes from. */
export interface Pr {
  /** The date of the workout, `YYYY-MM-DD`. */
  date: string;
  /** For pull-ups the reps are the PR. */
  reps: number;
  /** Kilograms such as `72.5`; null for pull-ups. */
  weight: string | null;
  /** Kilograms such as `81.7`; null for pull-ups. */
  estimated_1rm: string | null;
}

/** The PRs of a standard exercise; null for a period without a set that counts. */
export interface ExercisePrs {
  exercise_id: string;
  days_90: Pr | null;
  all_time: Pr | null;
}

/** The PRs of the signed-in user in every standard exercise. */
export async function fetchPrs(): Promise<ExercisePrs[]> {
  const response = await apiFetch("GET", "/api/prs");
  return (await response.json()) as ExercisePrs[];
}

/** The PR itself: `120 kg` for the Big Three, `9 reps` for pull-ups. */
export function prValue(pr: Pr): string {
  if (pr.estimated_1rm !== null) return `${pr.estimated_1rm} kg`;
  return pr.reps === 1 ? "1 rep" : `${pr.reps} reps`;
}

/** The set a Big Three PR comes from, as `90×10`; null for pull-ups, whose reps are the PR. */
export function prSet(pr: Pr): string | null {
  return pr.weight === null ? null : `${pr.weight}×${pr.reps}`;
}
