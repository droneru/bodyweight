import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { errorMessage, UnauthorizedError } from "./api";
import { toIsoDate } from "./dates";
import { exerciseNamer } from "./exerciseNames";
import { fetchExercises } from "./exercises";
import { fetchWorkouts, newWorkoutId, putWorkout } from "./workouts";
import type { WorkoutSummary } from "./workouts";

/** The Workouts screen: the list of workouts, newest first, and starting a new one. */
export function WorkoutsScreen({ onUnauthorized }: { onUnauthorized: () => void }) {
  // null: not loaded yet.
  const [workouts, setWorkouts] = useState<WorkoutSummary[] | null>(null);
  const [nameOf, setNameOf] = useState<(id: string) => string>(() => () => "");
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  const fail = useCallback(
    (e: unknown, fallback: string) => {
      if (e instanceof UnauthorizedError) onUnauthorized();
      else setError(errorMessage(e, fallback));
    },
    [onUnauthorized],
  );

  useEffect(() => {
    Promise.all([fetchWorkouts(), fetchExercises()]).then(
      ([list, exercises]) => {
        setNameOf(() => exerciseNamer(exercises));
        setWorkouts(list);
      },
      (e: unknown) => fail(e, "Could not load workouts."),
    );
  }, [fail]);

  /** The workout is created at once, then opened for editing. */
  async function handleNew() {
    setCreating(true);
    setError(null);
    try {
      const id = newWorkoutId();
      await putWorkout(id, { date: toIsoDate(new Date()), note: "", entries: [] });
      await navigate(`/workouts/${id}`);
    } catch (e) {
      fail(e, "Could not start a workout. Please try again.");
      setCreating(false);
    }
  }

  return (
    <section>
      <div className="mt-6 mb-4 flex items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">Workouts</h1>
        <Button
          className="h-11 px-5 text-base font-semibold"
          onClick={handleNew}
          disabled={creating}
        >
          New workout
        </Button>
      </div>
      {error && (
        <p className="mb-4 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}

      {workouts?.length === 0 && <p className="my-6 text-muted-foreground">No workouts yet.</p>}
      {workouts && workouts.length > 0 && (
        <Card className="py-0">
          <ul className="divide-y">
            {workouts.map((w) => (
              <li key={w.id}>
                <Link
                  to={`/workouts/${w.id}`}
                  className="block px-3.5 py-3 hover:bg-muted/50 focus-visible:bg-muted/50 focus-visible:outline-none"
                >
                  <div className="flex gap-3">
                    <span className="shrink-0 tabular-nums">{w.date}</span>
                    <span className="truncate text-muted-foreground">
                      {w.exercise_ids.map(nameOf).join(", ") || "No exercises"}
                    </span>
                  </div>
                  {w.note && (
                    <p className="mt-0.5 truncate text-sm text-muted-foreground">{w.note}</p>
                  )}
                </Link>
              </li>
            ))}
          </ul>
        </Card>
      )}
    </section>
  );
}
