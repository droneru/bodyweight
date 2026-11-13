import { expect, test } from "vitest";
import { addDays, toIsoDate } from "./dates";

test("formats the local date with zero padding", () => {
  expect(toIsoDate(new Date(2026, 0, 5, 23, 59))).toBe("2026-01-05");
  expect(toIsoDate(new Date(2026, 10, 13, 0, 0))).toBe("2026-11-13");
});

test("adds days across month and year ends", () => {
  expect(toIsoDate(addDays(new Date(2026, 11, 30), 2))).toBe("2027-01-01");
  expect(toIsoDate(addDays(new Date(2028, 1, 28), 2))).toBe("2028-03-01");
});
