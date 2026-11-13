import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { ChevronLeftIcon, XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  NativeSelect,
  NativeSelectOptGroup,
  NativeSelectOption,
} from "@/components/ui/native-select";
import { Textarea } from "@/components/ui/textarea";
import { errorMessage, UnauthorizedError } from "./api";
import { addDays, toIsoDate } from "./dates";
import { exerciseNamer } from "./exerciseNames";
import { fetchExercises } from "./exercises";
import type { Exercises } from "./exercises";
import { useAutosave } from "./useAutosave";
import {
  hasInvalidReps,
  hasInvalidWeight,
  isBlank,
  newEntry,
  newSet,
  toDraft,
  toWorkout,
} from "./workoutDraft";
import type { Draft, DraftEntry, DraftSet } from "./workoutDraft";
import {
  deleteWorkout,
  fetchWorkout,
  MAX_DAYS_AHEAD,
  MAX_ENTRIES,
  MAX_NOTE_LENGTH,
  MAX_SETS,
  putWorkout,
} from "./workouts";
import type { Workout } from "./workouts";

/** Form fields tall enough for a thumb. */
const FIELD = "h-11 px-3 tabular-nums";

/** The Workout screen: view and edit one workout. Every change is saved at once. */
export function WorkoutScreen({ onUnauthorized }: { onUnauthorized: () => void }) {
  const { id = "" } = useParams();
  // undefined: not loaded yet; null: there is no such workout.
  const [loaded, setLoaded] = useState<{ workout: Workout; exercises: Exercises } | null>();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([fetchWorkout(id), fetchExercises()]).then(
      ([workout, exercises]) => setLoaded(workout && { workout, exercises }),
      (e: unknown) => {
        if (e instanceof UnauthorizedError) onUnauthorized();
        else setError("Could not load the workout.");
      },
    );
  }, [id, onUnauthorized]);

  return (
    <section>
      <Link
        to="/workouts"
        className="mt-4 -ml-1 inline-flex items-center gap-0.5 py-2 text-sm text-muted-foreground"
      >
        <ChevronLeftIcon className="size-4" aria-hidden="true" />
        Workouts
      </Link>
      {error && (
        <p className="mt-2.5 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      {loaded === null && <p className="my-6 text-muted-foreground">There is no such workout.</p>}
      {loaded && (
        <WorkoutEditor
          key={id}
          id={id}
          initial={loaded.workout}
          exercises={loaded.exercises}
          onUnauthorized={onUnauthorized}
        />
      )}
    </section>
  );
}

function WorkoutEditor({
  id,
  initial,
  exercises,
  onUnauthorized,
}: {
  id: string;
  initial: Workout;
  exercises: Exercises;
  onUnauthorized: () => void;
}) {
  const [draft, setDraft] = useState<Draft>(() => toDraft(initial));
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const navigate = useNavigate();
  const nameOf = useMemo(() => exerciseNamer(exercises), [exercises]);

  const workout = useMemo(() => toWorkout(draft), [draft]);
  const save = useCallback((w: Workout) => putWorkout(id, w), [id]);
  const autosave = useAutosave(workout, initial, save);

  const unauthorized = autosave.error instanceof UnauthorizedError;
  useEffect(() => {
    if (unauthorized) onUnauthorized();
  }, [unauthorized, onUnauthorized]);

  function setEntry(key: number, change: (entry: DraftEntry) => DraftEntry) {
    setDraft((d) => ({ ...d, entries: d.entries.map((e) => (e.key === key ? change(e) : e)) }));
  }

  function setSet(entryKey: number, key: number, change: Partial<DraftSet>) {
    setEntry(entryKey, (e) => ({
      ...e,
      sets: e.sets.map((s) => (s.key === key ? { ...s, ...change } : s)),
    }));
  }

  function addEntry(exerciseId: string) {
    setDraft((d) => ({ ...d, entries: [...d.entries, newEntry(exerciseId, [newSet()])] }));
  }

  function removeEntry(entry: DraftEntry) {
    const filled = entry.sets.some((s) => !isBlank(s));
    if (filled && !window.confirm(`Remove ${nameOf(entry.exerciseId)} with its sets?`)) return;
    setDraft((d) => ({ ...d, entries: d.entries.filter((e) => e.key !== entry.key) }));
  }

  async function handleDelete() {
    if (!window.confirm("Delete this workout?")) return;
    setDeleting(true);
    setDeleteError(null);
    try {
      // A save that lands after the delete would bring the workout back.
      await autosave.pause();
      await deleteWorkout(id);
      await navigate("/workouts");
    } catch (e) {
      if (e instanceof UnauthorizedError) onUnauthorized();
      else setDeleteError("Could not delete the workout.");
      setDeleting(false);
      autosave.resume();
    }
  }

  const available = exercises.custom.filter((e) => !e.archived);

  return (
    <>
      <div className="mt-1 mb-4 flex items-baseline justify-between gap-3">
        <h1 className="text-2xl font-bold">Workout</h1>
        <SaveStatus invalid={workout === null} autosave={autosave} />
      </div>

      <div className="grid gap-3">
        <div className="grid gap-1.5">
          <Label htmlFor="workout-date" className="font-normal text-muted-foreground">
            Date
          </Label>
          <Input
            id="workout-date"
            className={FIELD}
            type="date"
            value={draft.date}
            max={toIsoDate(addDays(new Date(), MAX_DAYS_AHEAD))}
            onChange={(e) => setDraft((d) => ({ ...d, date: e.target.value }))}
            aria-invalid={draft.date === ""}
            required
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="workout-note" className="font-normal text-muted-foreground">
            Note
          </Label>
          <Textarea
            id="workout-note"
            className="px-3"
            value={draft.note}
            maxLength={MAX_NOTE_LENGTH}
            onChange={(e) => setDraft((d) => ({ ...d, note: e.target.value }))}
          />
        </div>
      </div>

      {draft.entries.map((entry) => (
        <Card key={entry.key} className="mt-4 gap-0 px-3.5 py-3">
          <div className="flex items-center justify-between gap-2">
            <h2 className="min-w-0 font-semibold break-words">{nameOf(entry.exerciseId)}</h2>
            <Button
              variant="ghost"
              className="-mr-2 h-9 px-2 text-muted-foreground"
              onClick={() => removeEntry(entry)}
            >
              Remove
            </Button>
          </div>

          {entry.sets.length > 0 && (
            <div className="mt-1 grid grid-cols-[1.5rem_1fr_1fr_2.75rem] items-center gap-x-2 gap-y-2">
              <span />
              <span className="text-sm text-muted-foreground">Reps</span>
              <span className="text-sm text-muted-foreground">Weight, kg</span>
              <span />
              {entry.sets.map((set, index) => (
                <SetRow
                  key={set.key}
                  number={index + 1}
                  set={set}
                  onChange={(change) => setSet(entry.key, set.key, change)}
                  onRemove={() =>
                    setEntry(entry.key, (e) => ({
                      ...e,
                      sets: e.sets.filter((s) => s.key !== set.key),
                    }))
                  }
                />
              ))}
            </div>
          )}
          <Button
            variant="outline"
            className="mt-3 h-11 text-base"
            onClick={() => setEntry(entry.key, (e) => ({ ...e, sets: [...e.sets, newSet()] }))}
            disabled={entry.sets.length >= MAX_SETS}
          >
            Add set
          </Button>
        </Card>
      ))}

      <NativeSelect
        className="mt-4 w-full [&_select]:h-11 [&_select]:pl-3"
        aria-label="Add exercise"
        value=""
        onChange={(e) => addEntry(e.target.value)}
        disabled={draft.entries.length >= MAX_ENTRIES}
      >
        <NativeSelectOption value="" disabled>
          Add exercise…
        </NativeSelectOption>
        {exercises.standard.map((e) => (
          <NativeSelectOption key={e.id} value={e.id}>
            {e.name}
          </NativeSelectOption>
        ))}
        {available.length > 0 && (
          <NativeSelectOptGroup label="My exercises">
            {available.map((e) => (
              <NativeSelectOption key={e.id} value={e.id}>
                {e.name}
              </NativeSelectOption>
            ))}
          </NativeSelectOptGroup>
        )}
      </NativeSelect>

      <div className="mt-10">
        <Button
          variant="destructive"
          className="h-11 px-4 text-base"
          onClick={handleDelete}
          disabled={deleting}
        >
          Delete workout
        </Button>
        {deleteError && (
          <p className="mt-2.5 text-sm text-destructive" role="alert">
            {deleteError}
          </p>
        )}
      </div>
    </>
  );
}

/** A set of an exercise entry: its number, reps, weight and the button that removes it. */
function SetRow({
  number,
  set,
  onChange,
  onRemove,
}: {
  number: number;
  set: DraftSet;
  onChange: (change: Partial<DraftSet>) => void;
  onRemove: () => void;
}) {
  return (
    <>
      <span className="text-sm text-muted-foreground tabular-nums">{number}</span>
      <Input
        className={FIELD}
        value={set.reps}
        onChange={(e) => onChange({ reps: e.target.value })}
        inputMode="numeric"
        autoComplete="off"
        aria-label={`Reps of set ${number}`}
        aria-invalid={hasInvalidReps(set)}
      />
      <Input
        className={FIELD}
        value={set.weight}
        onChange={(e) => onChange({ weight: e.target.value })}
        inputMode="decimal"
        autoComplete="off"
        placeholder="bodyweight"
        aria-label={`Weight of set ${number}, kg`}
        aria-invalid={hasInvalidWeight(set)}
      />
      <Button
        variant="ghost"
        className="size-11 text-muted-foreground"
        onClick={onRemove}
        aria-label={`Remove set ${number}`}
      >
        <XIcon className="size-5" aria-hidden="true" />
      </Button>
    </>
  );
}

/** Tells whether the workout on the screen is the one on the server. */
function SaveStatus({
  invalid,
  autosave,
}: {
  invalid: boolean;
  autosave: ReturnType<typeof useAutosave>;
}) {
  if (invalid) {
    return (
      <span className="text-right text-sm text-destructive" role="status">
        Not saved: check the fields
      </span>
    );
  }
  if (autosave.status === "failed") {
    return (
      <span className="text-right text-sm text-destructive" role="alert">
        {errorMessage(autosave.error, "Not saved")}.{" "}
        <button type="button" className="underline" onClick={autosave.retry}>
          Retry
        </button>
      </span>
    );
  }
  return (
    <span className="text-sm text-muted-foreground" role="status">
      {autosave.status === "saving" ? "Saving…" : "Saved"}
    </span>
  );
}
