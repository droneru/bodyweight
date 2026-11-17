import { expect, test } from "vitest";
import { ApiError, UnauthorizedError } from "./api";
import {
  clearUnsaved,
  flushUnsaved,
  listUnsaved,
  loadUnsaved,
  storeUnsaved,
} from "./unsavedWorkouts";
import type { KeyValueStorage } from "./unsavedWorkouts";
import type { Workout } from "./workouts";

function fakeStorage(): KeyValueStorage {
  const items = new Map<string, string>();
  return {
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => void items.set(key, value),
    removeItem: (key) => void items.delete(key),
    key: (index) => [...items.keys()][index] ?? null,
    get length() {
      return items.size;
    },
  };
}

function workout(note: string): Workout {
  return { date: "2026-11-01", note, entries: [] };
}

test("an unsaved workout is kept until it is cleared", () => {
  const storage = fakeStorage();
  expect(loadUnsaved("alice", "a1", storage)).toBeNull();

  storeUnsaved("alice", "a1", workout("first"), storage);
  storeUnsaved("alice", "a1", workout("second"), storage);
  expect(loadUnsaved("alice", "a1", storage)).toEqual(workout("second"));

  clearUnsaved("alice", "a1", undefined, storage);
  expect(loadUnsaved("alice", "a1", storage)).toBeNull();
});

test("clearing a version that was replaced meanwhile keeps the newer one", () => {
  const storage = fakeStorage();
  storeUnsaved("alice", "a1", workout("newer"), storage);
  clearUnsaved("alice", "a1", workout("older"), storage);
  expect(loadUnsaved("alice", "a1", storage)).toEqual(workout("newer"));
  clearUnsaved("alice", "a1", workout("newer"), storage);
  expect(loadUnsaved("alice", "a1", storage)).toBeNull();
});

test("unsaved workouts are kept per user", () => {
  const storage = fakeStorage();
  storage.setItem("something-else", "{}");
  storeUnsaved("alice", "a1", workout("alice 1"), storage);
  storeUnsaved("alice", "a2", workout("alice 2"), storage);
  storeUnsaved("bob", "a1", workout("bob"), storage);

  expect(listUnsaved("alice", storage)).toEqual([
    { id: "a1", workout: workout("alice 1") },
    { id: "a2", workout: workout("alice 2") },
  ]);
  expect(loadUnsaved("bob", "a1", storage)).toEqual(workout("bob"));
  expect(listUnsaved("carol", storage)).toEqual([]);
});

test("flushing sends the unsaved workouts and forgets the ones that got through", async () => {
  const storage = fakeStorage();
  storeUnsaved("alice", "a1", workout("one"), storage);
  storeUnsaved("alice", "a2", workout("two"), storage);
  storeUnsaved("bob", "b1", workout("bob"), storage);

  const sent: string[] = [];
  const save = (id: string) => {
    sent.push(id);
    return id === "a2" ? Promise.reject(new TypeError("offline")) : Promise.resolve();
  };
  expect(await flushUnsaved("alice", save, storage)).toBe(1);
  expect(sent).toEqual(["a1", "a2"]);
  expect(listUnsaved("alice", storage)).toEqual([{ id: "a2", workout: workout("two") }]);
  expect(listUnsaved("bob", storage)).toHaveLength(1);

  expect(await flushUnsaved("alice", () => Promise.resolve(), storage)).toBe(0);
  expect(listUnsaved("alice", storage)).toEqual([]);
});

test("a workout the server refuses as invalid is dropped, a server failure is not", async () => {
  const storage = fakeStorage();
  storeUnsaved("alice", "a1", workout("invalid"), storage);
  storeUnsaved("alice", "a2", workout("unlucky"), storage);
  const save = (id: string) => Promise.reject(new ApiError(id === "a1" ? 400 : 500, "refused"));

  expect(await flushUnsaved("alice", save, storage)).toBe(1);
  expect(listUnsaved("alice", storage)).toEqual([{ id: "a2", workout: workout("unlucky") }]);
});

test("without a session nothing is dropped", async () => {
  const storage = fakeStorage();
  storeUnsaved("alice", "a1", workout("one"), storage);
  const save = () => Promise.reject(new UnauthorizedError());
  await expect(flushUnsaved("alice", save, storage)).rejects.toBeInstanceOf(UnauthorizedError);
  expect(listUnsaved("alice", storage)).toHaveLength(1);
});
