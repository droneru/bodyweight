import type { Workout } from "./workouts";

/** A workout as it is being edited: reps and weights are what the user typed so far. */
export interface Draft {
  date: string;
  note: string;
  entries: DraftEntry[];
}

export interface DraftEntry {
  /** Identifies the entry among the entries of the draft, for React. */
  key: number;
  exerciseId: string;
  sets: DraftSet[];
}

export interface DraftSet {
  /** Identifies the set among the sets of the draft, for React. */
  key: number;
  reps: string;
  /** Empty for a set done with bodyweight only. */
  weight: string;
}

const MAX_REPS = 1000;
const MAX_WEIGHT = 1000;

let lastKey = 0;

export function newSet(reps = "", weight = ""): DraftSet {
  return { key: ++lastKey, reps, weight };
}

export function newEntry(exerciseId: string, sets: DraftSet[]): DraftEntry {
  return { key: ++lastKey, exerciseId, sets };
}

export function toDraft(workout: Workout): Draft {
  return {
    date: workout.date,
    note: workout.note,
    entries: workout.entries.map((entry) =>
      newEntry(
        entry.exercise_id,
        entry.sets.map((set) => newSet(String(set.reps), set.weight ?? "")),
      ),
    ),
  };
}

/** A row the user has not filled in yet. It is not a set and is not saved. */
export function isBlank(set: DraftSet): boolean {
  return set.reps.trim() === "" && set.weight.trim() === "";
}

/** The reps of the set, or null if they are not an integer from 1 to 1000. */
export function parseReps(input: string): number | null {
  const text = input.trim();
  if (!/^\d{1,4}$/.test(text)) return null;
  const reps = Number(text);
  return reps >= 1 && reps <= MAX_REPS ? reps : null;
}

/**
 * The weight as the server takes it. Returns null for no weight and undefined if the input
 * is not a weight: more than 0, at most 1000 kg, in steps of 0.25. A decimal comma is fine.
 */
export function parseWeight(input: string): string | null | undefined {
  const text = input.trim().replace(",", ".");
  if (text === "") return null;
  if (!/^\d{1,4}(\.\d{1,2})?$/.test(text)) return undefined;
  const quarters = Number(text) * 4;
  if (!Number.isInteger(quarters) || quarters <= 0 || quarters > MAX_WEIGHT * 4) return undefined;
  return text;
}

/** Whether a field of the set keeps the workout from being saved. */
export function hasInvalidReps(set: DraftSet): boolean {
  return !isBlank(set) && parseReps(set.reps) === null;
}

export function hasInvalidWeight(set: DraftSet): boolean {
  return parseWeight(set.weight) === undefined;
}

/**
 * The workout to save, or null if the draft cannot be saved as it is: it has no date, or
 * a set with invalid reps or weight. Blank rows are left out.
 */
export function toWorkout(draft: Draft): Workout | null {
  if (draft.date === "") return null;
  const entries = [];
  for (const entry of draft.entries) {
    const sets = [];
    for (const set of entry.sets) {
      if (isBlank(set)) continue;
      const reps = parseReps(set.reps);
      const weight = parseWeight(set.weight);
      if (reps === null || weight === undefined) return null;
      sets.push({ reps, weight });
    }
    entries.push({ exercise_id: entry.exerciseId, sets });
  }
  return { date: draft.date, note: draft.note, entries };
}
