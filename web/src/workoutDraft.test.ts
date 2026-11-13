import { expect, test } from "vitest";
import { newEntry, newSet, parseReps, parseWeight, toDraft, toWorkout } from "./workoutDraft";
import type { Draft } from "./workoutDraft";
import type { Workout } from "./workouts";

function draft(...sets: [reps: string, weight: string][]): Draft {
  const rows = sets.map(([reps, weight]) => newSet(reps, weight));
  return { date: "2026-11-01", note: "legs", entries: [newEntry("squat", rows)] };
}

test("reps are an integer from 1 to 1000", () => {
  expect(parseReps("5")).toBe(5);
  expect(parseReps(" 1000 ")).toBe(1000);
  for (const input of ["", "0", "1001", "5.5", "-1", "five"]) {
    expect(parseReps(input), input).toBeNull();
  }
});

test("a weight is in steps of 0.25, with a point or a comma", () => {
  expect(parseWeight("70")).toBe("70");
  expect(parseWeight(" 72,5 ")).toBe("72.5");
  expect(parseWeight("71.25")).toBe("71.25");
  expect(parseWeight("1000")).toBe("1000");
});

test("no weight is not an invalid weight", () => {
  expect(parseWeight("")).toBeNull();
  expect(parseWeight("  ")).toBeNull();
  for (const input of ["0", "70.1", "70.125", "1000.25", "70.", "abc", "7 0"]) {
    expect(parseWeight(input), input).toBeUndefined();
  }
});

test("a draft becomes the workout to save", () => {
  expect(toWorkout(draft(["5", "60"], ["12", ""], ["3", "72,5"]))).toEqual({
    date: "2026-11-01",
    note: "legs",
    entries: [
      {
        exercise_id: "squat",
        sets: [
          { reps: 5, weight: "60" },
          { reps: 12, weight: null },
          { reps: 3, weight: "72.5" },
        ],
      },
    ],
  });
});

test("blank rows are not saved, the entry is", () => {
  expect(toWorkout(draft(["", ""], ["5", "60"], [" ", " "]))?.entries).toEqual([
    { exercise_id: "squat", sets: [{ reps: 5, weight: "60" }] },
  ]);
  expect(toWorkout(draft(["", ""]))?.entries).toEqual([{ exercise_id: "squat", sets: [] }]);
});

test("a draft with an invalid set or without a date is not saved", () => {
  expect(toWorkout(draft(["5", "60"], ["", "70"]))).toBeNull();
  expect(toWorkout(draft(["0", ""]))).toBeNull();
  expect(toWorkout(draft(["5", "70.1"]))).toBeNull();
  expect(toWorkout({ ...draft(["5", "60"]), date: "" })).toBeNull();
});

test("a saved workout survives the round trip through a draft", () => {
  const workout: Workout = {
    date: "2026-11-01",
    note: "",
    entries: [
      { exercise_id: "pull-ups", sets: [{ reps: 9, weight: null }] },
      { exercise_id: "squat", sets: [{ reps: 5, weight: "72.5" }] },
      { exercise_id: "deadlift", sets: [] },
    ],
  };
  expect(toWorkout(toDraft(workout))).toEqual(workout);
});
