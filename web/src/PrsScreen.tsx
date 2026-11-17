import { useEffect, useState } from "react";
import { Card } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { UnauthorizedError } from "./api";
import { exerciseNamer } from "./exerciseNames";
import { fetchExercises } from "./exercises";
import { fetchPrs, prSet, prValue } from "./prs";
import type { ExercisePrs, Pr } from "./prs";

/** The PRs screen: the standard exercises by period. */
export function PrsScreen({ onUnauthorized }: { onUnauthorized: () => void }) {
  // null: not loaded yet.
  const [prs, setPrs] = useState<ExercisePrs[] | null>(null);
  const [nameOf, setNameOf] = useState<(id: string) => string>(() => () => "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([fetchPrs(), fetchExercises()]).then(
      ([list, exercises]) => {
        setNameOf(() => exerciseNamer(exercises));
        setPrs(list);
      },
      (e: unknown) => {
        if (e instanceof UnauthorizedError) onUnauthorized();
        else setError("Could not load PRs.");
      },
    );
  }, [onUnauthorized]);

  return (
    <section>
      <h1 className="mt-6 mb-4 text-2xl font-bold">PRs</h1>
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      {prs && (
        <>
          <Card className="py-0">
            <Table className="text-base">
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead className="px-3.5 text-muted-foreground">Exercise</TableHead>
                  <TableHead className="px-2 text-muted-foreground">90 days</TableHead>
                  <TableHead className="px-3.5 text-muted-foreground">All time</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {prs.map((row) => (
                  <TableRow key={row.exercise_id} className="hover:bg-transparent">
                    <TableCell className="px-3.5 py-3 align-top whitespace-normal">
                      {nameOf(row.exercise_id)}
                    </TableCell>
                    <PrCell className="px-2" pr={row.days_90} />
                    <PrCell className="px-3.5" pr={row.all_time} />
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </Card>
          <p className="mt-3 text-sm text-muted-foreground">
            Pull-ups: the most reps in a set without a weight. The others: the estimated 1RM of a
            set of up to 12 reps.
          </p>
        </>
      )}
    </section>
  );
}

/** A PR with the set and the date it comes from, or a dash if there is none. */
function PrCell({ pr, className }: { pr: Pr | null; className: string }) {
  return (
    <TableCell className={`${className} py-3 align-top whitespace-normal tabular-nums`}>
      {pr ? (
        <>
          <div className="font-semibold">{prValue(pr)}</div>
          <div className="text-sm text-muted-foreground">{prSet(pr)}</div>
          <div className="text-sm whitespace-nowrap text-muted-foreground">{pr.date}</div>
        </>
      ) : (
        <span className="text-muted-foreground" aria-label="No PR">
          —
        </span>
      )}
    </TableCell>
  );
}
