import type { Exercises } from "./exercises";

/** Shown for an exercise that is not in the catalog any more. */
const UNKNOWN = "Deleted exercise";

/** Looks up the name of an exercise, standard or custom, by its id. */
export function exerciseNamer(exercises: Exercises): (id: string) => string {
  const names = new Map<string, string>();
  for (const e of [...exercises.standard, ...exercises.custom]) names.set(e.id, e.name);
  return (id) => names.get(id) ?? UNKNOWN;
}
