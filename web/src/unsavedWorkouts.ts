import { ApiError, UnauthorizedError } from "./api";
import { putWorkout, settleSaves } from "./workouts";
import type { Workout } from "./workouts";

/**
 * Workouts with changes the server has not confirmed, kept in localStorage so that they
 * survive a lost connection and a closed tab. They are kept per user: the next user of the
 * browser must not get them.
 */
const PREFIX = "bodyweight:unsaved-workout:";

/** The part of the Web Storage API used here. */
export type KeyValueStorage = Pick<
  Storage,
  "getItem" | "setItem" | "removeItem" | "key" | "length"
>;

function keyOf(login: string, id: string): string {
  return `${PREFIX}${login}:${id}`;
}

/** Remembers the workout as not saved yet, in place of an earlier version of it. */
export function storeUnsaved(
  login: string,
  id: string,
  workout: Workout,
  storage: KeyValueStorage = localStorage,
): void {
  try {
    storage.setItem(keyOf(login, id), JSON.stringify(workout));
  } catch {
    // The storage is full or turned off: the change is only as safe as the open page.
  }
}

/** Forgets the unsaved workout. With `only`, just if that is the version remembered. */
export function clearUnsaved(
  login: string,
  id: string,
  only?: Workout,
  storage: KeyValueStorage = localStorage,
): void {
  try {
    const key = keyOf(login, id);
    if (only === undefined || storage.getItem(key) === JSON.stringify(only)) {
      storage.removeItem(key);
    }
  } catch {
    // Nothing was stored either.
  }
}

/** The unsaved version of the workout, or null if the server has the latest one. */
export function loadUnsaved(
  login: string,
  id: string,
  storage: KeyValueStorage = localStorage,
): Workout | null {
  return listUnsaved(login, storage).find((u) => u.id === id)?.workout ?? null;
}

/** All unsaved workouts of the user. */
export function listUnsaved(
  login: string,
  storage: KeyValueStorage = localStorage,
): { id: string; workout: Workout }[] {
  const prefix = keyOf(login, "");
  const unsaved = [];
  try {
    for (let i = 0; i < storage.length; i++) {
      const key = storage.key(i);
      if (!key?.startsWith(prefix)) continue;
      const workout = JSON.parse(storage.getItem(key) ?? "null") as Workout | null;
      if (workout) unsaved.push({ id: key.slice(prefix.length), workout });
    }
  } catch {
    // Unreadable storage holds nothing to send.
  }
  return unsaved;
}

/**
 * Sends the unsaved workouts of the user to the server and returns how many are still not
 * saved. `save` is `putWorkout`, replaced in tests.
 */
export async function flushUnsaved(
  login: string,
  save: (id: string, workout: Workout) => Promise<void> = putWorkout,
  storage: KeyValueStorage = localStorage,
): Promise<number> {
  // A workout that was just left may be on its way to the server already.
  await settleSaves();
  let left = 0;
  for (const { id, workout } of listUnsaved(login, storage)) {
    try {
      await save(id, workout);
      clearUnsaved(login, id, workout, storage);
    } catch (e) {
      if (e instanceof UnauthorizedError) throw e;
      // What the server refuses as invalid will not be accepted later either.
      if (e instanceof ApiError && e.status === 400) clearUnsaved(login, id, workout, storage);
      else left++;
    }
  }
  return left;
}
