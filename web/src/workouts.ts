import { ApiError, apiFetch } from "./api";

/** A workout in the list, as returned by `GET /api/workouts`. */
export interface WorkoutSummary {
  id: string;
  /** `YYYY-MM-DD`. */
  date: string;
  /** Empty if there is no note. */
  note: string;
  /** The exercises of the workout in the order they first appear, without repeats. */
  exercise_ids: string[];
}

/** A whole workout, as returned and accepted by `/api/workouts/{id}`. */
export interface Workout {
  /** `YYYY-MM-DD`. */
  date: string;
  /** Empty if there is no note. */
  note: string;
  entries: ExerciseEntry[];
}

/** One exercise performed within a workout, together with its sets. */
export interface ExerciseEntry {
  exercise_id: string;
  sets: WorkoutSet[];
}

export interface WorkoutSet {
  reps: number;
  /** Kilograms such as `72.5`; null for a set done with bodyweight only. */
  weight: string | null;
}

/** How far past today the server accepts a workout. */
export const MAX_DAYS_AHEAD = 2;
/** Size limits of the server. */
export const MAX_NOTE_LENGTH = 2000;
export const MAX_ENTRIES = 50;
export const MAX_SETS = 50;

/** Saves under way. */
const saving = new Set<Promise<unknown>>();

/** All workouts of the signed-in user, newest first. */
export async function fetchWorkouts(): Promise<WorkoutSummary[]> {
  // Leaving a workout saves its last change; the list waits, to show the workout as it was left.
  await Promise.allSettled(saving);
  const response = await apiFetch("GET", "/api/workouts");
  return (await response.json()) as WorkoutSummary[];
}

/** The workout, or null if the user has no workout with this id. */
export async function fetchWorkout(id: string): Promise<Workout | null> {
  try {
    const response = await apiFetch("GET", workoutPath(id));
    return (await response.json()) as Workout;
  } catch (e) {
    if (e instanceof ApiError && e.status === 404) return null;
    throw e;
  }
}

/** Creates the workout with this id or replaces it as a whole. */
export async function putWorkout(id: string, workout: Workout): Promise<void> {
  const request = apiFetch("PUT", workoutPath(id), workout);
  saving.add(request);
  try {
    await request;
  } finally {
    saving.delete(request);
  }
}

export async function deleteWorkout(id: string): Promise<void> {
  await apiFetch("DELETE", workoutPath(id));
}

/** A random id for a new workout: 32 lowercase hex digits. */
export function newWorkoutId(): string {
  // crypto.randomUUID needs a secure context, which a dev server opened from a phone is not.
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

function workoutPath(id: string): string {
  return `/api/workouts/${encodeURIComponent(id)}`;
}
