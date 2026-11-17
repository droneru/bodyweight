import { expect, test } from "vitest";
import { prSet, prValue } from "./prs";

test("a Big Three PR is the estimated 1RM, with the set and the date", () => {
  const pr = { date: "2026-10-12", reps: 10, weight: "90", estimated_1rm: "120" };
  expect(`${prValue(pr)} — ${prSet(pr)}, ${pr.date}`).toBe("120 kg — 90×10, 2026-10-12");
});

test("a pull-ups PR is the reps, with the date", () => {
  const pr = { date: "2026-10-18", reps: 9, weight: null, estimated_1rm: null };
  expect(prValue(pr)).toBe("9 reps");
  expect(prSet(pr)).toBeNull();
  expect(prValue({ ...pr, reps: 1 })).toBe("1 rep");
});
