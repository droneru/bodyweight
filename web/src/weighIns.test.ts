import { expect, test } from "vitest";
import { normalizeBodyweight } from "./weighIns";

test("turns a decimal comma into a point", () => {
  expect(normalizeBodyweight("82,5")).toBe("82.5");
});

test("trims and keeps valid input as is", () => {
  expect(normalizeBodyweight(" 82.5 ")).toBe("82.5");
  expect(normalizeBodyweight("82")).toBe("82");
});

test("does not hide malformed input from the server", () => {
  expect(normalizeBodyweight("1,234,5")).toBe("1.234,5");
  expect(normalizeBodyweight("abc")).toBe("abc");
});
