import { apiFetch } from "./api";

/** An exercise defined by the app, available to every user. */
export interface StandardExercise {
  id: string;
  name: string;
}

/** An exercise the user created for themselves. */
export interface CustomExercise {
  id: string;
  name: string;
  /** An archived exercise is not offered for new workouts but stays in the old ones. */
  archived: boolean;
}

/** What `GET /api/exercises` returns; `custom` is ordered by name. */
export interface Exercises {
  standard: StandardExercise[];
  custom: CustomExercise[];
}

/** The longest name the server accepts. */
export const MAX_NAME_LENGTH = 60;

/** The standard exercises and the custom exercises of the signed-in user. */
export async function fetchExercises(): Promise<Exercises> {
  const response = await apiFetch("GET", "/api/exercises");
  return (await response.json()) as Exercises;
}

export async function createExercise(name: string): Promise<CustomExercise> {
  const response = await apiFetch("POST", "/api/exercises", { name });
  return (await response.json()) as CustomExercise;
}

/** Renames and/or (un)archives a custom exercise; absent fields are left as they are. */
export async function updateExercise(
  id: string,
  changes: { name?: string; archived?: boolean },
): Promise<void> {
  await apiFetch("PATCH", `/api/exercises/${encodeURIComponent(id)}`, changes);
}

export async function deleteExercise(id: string): Promise<void> {
  await apiFetch("DELETE", `/api/exercises/${encodeURIComponent(id)}`);
}
