import { useCallback, useEffect, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ApiError, UnauthorizedError } from "./api";
import {
  createExercise,
  deleteExercise,
  fetchExercises,
  MAX_NAME_LENGTH,
  updateExercise,
} from "./exercises";
import type { CustomExercise } from "./exercises";

/** Form fields tall enough for a thumb. */
const FIELD = "h-11 px-3";

/** The My exercises screen: create, rename, archive and delete custom exercises. */
export function MyExercisesScreen({ onUnauthorized }: { onUnauthorized: () => void }) {
  // null: not loaded yet.
  const [exercises, setExercises] = useState<CustomExercise[] | null>(null);
  const [name, setName] = useState("");
  // The exercise being renamed and the name typed so far.
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fail = useCallback(
    (e: unknown, fallback: string) => {
      if (e instanceof UnauthorizedError) onUnauthorized();
      else setError(e instanceof ApiError && e.status === 400 ? capitalize(e.message) : fallback);
    },
    [onUnauthorized],
  );

  useEffect(() => {
    fetchExercises().then(
      (all) => setExercises(all.custom),
      (e: unknown) => fail(e, "Could not load exercises."),
    );
  }, [fail]);

  /** Runs a change and reloads the list. Returns whether the change went through. */
  async function change(action: () => Promise<unknown>, fallback: string): Promise<boolean> {
    setBusy(true);
    setError(null);
    try {
      await action();
      setExercises((await fetchExercises()).custom);
      return true;
    } catch (e) {
      fail(e, fallback);
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function handleCreate(event: FormEvent) {
    event.preventDefault();
    if (await change(() => createExercise(name), "Could not add the exercise. Please try again.")) {
      setName("");
    }
  }

  async function handleRename(event: FormEvent) {
    event.preventDefault();
    if (!renaming) return;
    const { id, name } = renaming;
    if (await change(() => updateExercise(id, { name }), "Could not rename the exercise.")) {
      setRenaming(null);
    }
  }

  function setArchived(exercise: CustomExercise, archived: boolean) {
    void change(
      () => updateExercise(exercise.id, { archived }),
      archived ? "Could not archive the exercise." : "Could not restore the exercise.",
    );
  }

  function handleDelete(exercise: CustomExercise) {
    if (!window.confirm(`Delete “${exercise.name}”?`)) return;
    void change(() => deleteExercise(exercise.id), "Could not delete the exercise.");
  }

  const active = exercises?.filter((e) => !e.archived) ?? [];
  const archived = exercises?.filter((e) => e.archived) ?? [];

  function row(exercise: CustomExercise): ReactNode {
    if (renaming?.id === exercise.id) {
      return (
        <li key={exercise.id} className="px-3.5 py-3">
          <form className="flex flex-wrap items-center gap-2" onSubmit={handleRename}>
            <Input
              className={`${FIELD} min-w-40 flex-1`}
              value={renaming.name}
              onChange={(e) => setRenaming({ id: exercise.id, name: e.target.value })}
              maxLength={MAX_NAME_LENGTH}
              aria-label="Exercise name"
              autoComplete="off"
              autoFocus
              required
            />
            <Button className="h-11 px-4" type="submit" disabled={busy}>
              Save
            </Button>
            <Button
              className="h-11 px-4"
              type="button"
              variant="ghost"
              onClick={() => setRenaming(null)}
            >
              Cancel
            </Button>
          </form>
        </li>
      );
    }
    return (
      <li key={exercise.id} className="flex flex-wrap items-center gap-x-1 px-3.5 py-2">
        <span
          className={`min-w-0 flex-1 basis-40 py-1 break-words ${exercise.archived ? "text-muted-foreground" : ""}`}
        >
          {exercise.name}
        </span>
        <div className="-mx-2 flex">
          {exercise.archived ? (
            <Action onClick={() => setArchived(exercise, false)} disabled={busy}>
              Restore
            </Action>
          ) : (
            <>
              <Action
                onClick={() => setRenaming({ id: exercise.id, name: exercise.name })}
                disabled={busy}
              >
                Rename
              </Action>
              <Action onClick={() => setArchived(exercise, true)} disabled={busy}>
                Archive
              </Action>
            </>
          )}
          <Action onClick={() => handleDelete(exercise)} disabled={busy} destructive>
            Delete
          </Action>
        </div>
      </li>
    );
  }

  return (
    <section>
      <h1 className="mt-6 mb-4 text-2xl font-bold">My exercises</h1>

      <form className="grid grid-cols-[1fr_auto] items-end gap-3" onSubmit={handleCreate}>
        <div className="grid min-w-0 gap-1.5">
          <Label htmlFor="exercise-name" className="font-normal text-muted-foreground">
            New exercise
          </Label>
          <Input
            id="exercise-name"
            className={FIELD}
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={MAX_NAME_LENGTH}
            autoComplete="off"
            placeholder="Overhead press"
            required
          />
        </div>
        <Button className="h-11 px-5 text-base font-semibold" type="submit" disabled={busy}>
          Add
        </Button>
      </form>
      {error && (
        <p className="mt-2.5 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}

      {exercises?.length === 0 && (
        <p className="my-6 text-muted-foreground">
          No exercises of your own yet. Pull-ups, bench press, squat and deadlift are always
          available.
        </p>
      )}
      {active.length > 0 && (
        <Card className="mt-6 py-0">
          <ul className="divide-y">{active.map(row)}</ul>
        </Card>
      )}
      {archived.length > 0 && (
        <>
          <h2 className="mt-6 mb-2 text-sm text-muted-foreground">Archived</h2>
          <Card className="py-0">
            <ul className="divide-y">{archived.map(row)}</ul>
          </Card>
        </>
      )}
    </section>
  );
}

/** A text button in an exercise row. */
function Action({
  onClick,
  disabled,
  destructive,
  children,
}: {
  onClick: () => void;
  disabled: boolean;
  destructive?: boolean;
  children: ReactNode;
}) {
  return (
    <Button
      variant="ghost"
      className={`h-9 px-2 ${destructive ? "text-destructive hover:text-destructive" : "text-muted-foreground"}`}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </Button>
  );
}

function capitalize(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}
