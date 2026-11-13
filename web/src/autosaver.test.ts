import { expect, test } from "vitest";
import { Autosaver } from "./autosaver";
import type { SaveStatus } from "./autosaver";

/** An autosaver of strings whose saves finish when the test says so. */
function setup(initial = "a") {
  const saves: { value: string; finish: () => void; fail: (e: unknown) => void }[] = [];
  const statuses: SaveStatus[] = [];
  const saver = new Autosaver<string>(
    initial,
    (value) =>
      new Promise<void>((resolve, reject) => saves.push({ value, finish: resolve, fail: reject })),
    (status) => statuses.push(status),
  );
  return { saver, saves, statuses, saved: () => saves.map((s) => s.value) };
}

/** Lets the pending promise callbacks run. */
const settle = () => new Promise((resolve) => setTimeout(resolve));

test("saves a changed value", async () => {
  const { saver, saves, statuses } = setup();
  expect(saver.set("b")).toBe(true);
  const flushed = saver.flush();
  expect(saves.map((s) => s.value)).toEqual(["b"]);
  saves[0].finish();
  await flushed;
  expect(statuses).toEqual(["saving", "saved"]);
});

test("does not save the saved value or a value that cannot be saved", async () => {
  const { saver, saved } = setup();
  expect(saver.set("a")).toBe(false);
  await saver.flush();
  expect(saver.set(null)).toBe(false);
  await saver.flush();
  expect(saved()).toEqual([]);
});

test("a flush with nothing to save does not block the next one", async () => {
  const { saver, saves, saved } = setup();
  await saver.flush();
  saver.set("b");
  const flushed = saver.flush();
  expect(saved()).toEqual(["b"]);
  saves[0].finish();
  await flushed;

  await saver.flush();
  saver.set("c");
  void saver.flush();
  expect(saved()).toEqual(["b", "c"]);
});

test("a value set during a save is saved next, the ones in between are skipped", async () => {
  const { saver, saves, saved, statuses } = setup();
  saver.set("b");
  const flushed = saver.flush();
  saver.set("c");
  saver.set("d");
  void saver.flush();
  expect(saved()).toEqual(["b"]);

  saves[0].finish();
  await settle();
  expect(saved()).toEqual(["b", "d"]);
  saves[1].finish();
  await flushed;
  expect(statuses.at(-1)).toBe("saved");

  // Going back to a value saved earlier is a change too.
  saver.set("b");
  void saver.flush();
  expect(saved()).toEqual(["b", "d", "b"]);
});

test("a failed save is reported and can be retried", async () => {
  const { saver, saves, saved } = setup();
  const errors: unknown[] = [];
  const failing = new Autosaver<string>(
    "a",
    () => Promise.reject(new Error("offline")),
    (status, error) => errors.push([status, error]),
  );
  failing.set("b");
  await failing.flush();
  expect(errors).toEqual([
    ["saving", null],
    ["failed", new Error("offline")],
  ]);

  saver.set("b");
  const flushed = saver.flush();
  saves[0].fail(new Error("offline"));
  await flushed;
  const retried = saver.flush();
  expect(saved()).toEqual(["b", "b"]);
  saves[1].finish();
  await retried;
});

test("pause waits for the save under way and holds back the next ones", async () => {
  const { saver, saves, saved } = setup();
  saver.set("b");
  void saver.flush();
  saver.set("c");

  let paused = false;
  const pausing = saver.pause().then(() => (paused = true));
  await settle();
  expect(paused).toBe(false);
  saves[0].finish();
  await pausing;

  await saver.flush();
  expect(saved()).toEqual(["b"]);

  saver.resume();
  expect(saved()).toEqual(["b", "c"]);
});
