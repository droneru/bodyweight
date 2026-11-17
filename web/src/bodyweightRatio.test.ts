import { expect, test } from "vitest";
import { bodyweightOn, bodyweightRatio } from "./bodyweightRatio";

test("the ratio is rounded down to tenths", () => {
  expect(bodyweightRatio("100", "80.0")).toBe("1.2"); // 1.25
  expect(bodyweightRatio("103.75", "80.0")).toBe("1.2"); // 1.297
  expect(bodyweightRatio("104", "80.0")).toBe("1.3");
  expect(bodyweightRatio("80", "80.0")).toBe("1.0");
  expect(bodyweightRatio("79.75", "80.0")).toBe("0.9");
  expect(bodyweightRatio("5", "82.5")).toBe("0.0");
  expect(bodyweightRatio("1000", "40.1")).toBe("24.9");
});

test("the bodyweight comes from the latest weigh-in up to the date", () => {
  const weighIns = [
    { date: "2026-11-10", bodyweight: "81.0" },
    { date: "2026-11-07", bodyweight: "82.5" },
    { date: "2026-11-03", bodyweight: "80.0" },
  ];
  expect(bodyweightOn(weighIns, "2026-11-02")).toBeNull();
  expect(bodyweightOn(weighIns, "2026-11-03")).toBe("80.0");
  expect(bodyweightOn(weighIns, "2026-11-06")).toBe("80.0");
  expect(bodyweightOn(weighIns, "2026-11-07")).toBe("82.5");
  expect(bodyweightOn(weighIns, "2026-11-30")).toBe("81.0");
  expect(bodyweightOn([], "2026-11-30")).toBeNull();
});
