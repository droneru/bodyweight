import type { WeighIn } from "./weighIns";

/**
 * The bodyweight to compare the sets of a workout on `date` with: from the latest weigh-in
 * on that date or before it, or null if there is none. `weighIns` are newest first.
 */
export function bodyweightOn(weighIns: WeighIn[], date: string): string | null {
  // Dates in the YYYY-MM-DD form compare as text.
  return weighIns.find((w) => w.date <= date)?.bodyweight ?? null;
}

/**
 * The weight of a set divided by the bodyweight, rounded down to tenths, e.g. `1.2`.
 * The same as `BodyweightRatio` of the domain crate: computed here as well, so that it
 * follows the weight and the date while they are edited.
 */
export function bodyweightRatio(weight: string, bodyweight: string): string {
  // In whole quarters and tenths of a kilogram, so that no rounding error creeps in.
  const quarters = Math.round(Number(weight) * 4);
  const tenths = Math.round(Number(bodyweight) * 10);
  const ratio = Math.floor((25 * quarters) / tenths);
  return `${Math.floor(ratio / 10)}.${ratio % 10}`;
}
