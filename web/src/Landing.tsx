import { useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { cn } from "cn";
import { Button, buttonVariants } from "@/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { devSignIn, YANDEX_SIGN_IN_URL } from "./auth";
import type { User } from "./auth";

/** The PRs table of the landing page: exercise, 90 days, all time. */
const EXAMPLE_PRS = [
  ["Pull-ups", "15 reps", "18 reps"],
  ["Bench press", "100 kg", "110 kg"],
  ["Squat", "120 kg", "140 kg"],
  ["Deadlift", "—", "180 kg"],
];

/** What signed-out visitors see: explains what the service is and offers to sign in. */
export function Landing({ onSignIn }: { onSignIn: (user: User) => void }) {
  const signInFailed = new URLSearchParams(window.location.search).get("sign_in") === "failed";

  return (
    <main>
      <section className="pt-8 pb-10">
        <h1 className="mb-4 text-[clamp(1.9rem,7vw,2.8rem)] leading-[1.15] font-bold">
          A simple strength-training log
        </h1>
        <p className="mb-7 text-lg text-muted-foreground">
          Record your workouts and weigh-ins. See your PRs in pull-ups and the Big Three for the
          last 90 days and all time.
        </p>
        <a
          className={cn(
            buttonVariants(),
            "h-13 w-full max-w-80 rounded-xl text-base font-semibold",
          )}
          href={YANDEX_SIGN_IN_URL}
        >
          Sign in with Yandex ID
        </a>
        {signInFailed && (
          <p className="mt-2.5 text-sm text-muted-foreground" role="alert">
            Sign-in failed. Please try again.
          </p>
        )}
        {import.meta.env.DEV && <DevSignIn onSignIn={onSignIn} />}
      </section>

      <section className="grid gap-3 sm:grid-cols-2">
        <Feature title="Workouts">
          Log exercises and sets: reps and an optional weight. When you add an exercise, the sets
          from last time are suggested — tick what you did, edit the rest.
        </Feature>
        <Feature title="PRs">
          Pull-ups: max reps in one set. Bench press, squat and deadlift: estimated 1RM by the
          Epley formula. Over the last 90 days and all time.
        </Feature>
        <Feature title="Weigh-ins">
          Track your bodyweight. Every weighted set shows its bodyweight ratio, e.g. bench press
          1.2× bodyweight.
        </Feature>
        <Feature title="My exercises">
          Add your own exercises beyond the standard ones. They are private to you.
        </Feature>
      </section>

      <section className="py-10" aria-label="PRs example">
        <h2 className="mb-3 text-xl font-bold">PRs at a glance</h2>
        <Card className="py-0">
          <Table className="text-base">
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead className="px-3.5" />
                <TableHead className="px-3.5 text-muted-foreground">90 days</TableHead>
                <TableHead className="px-3.5 text-muted-foreground">All time</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {EXAMPLE_PRS.map(([exercise, recent, allTime]) => (
                <TableRow key={exercise} className="hover:bg-transparent">
                  <TableHead scope="row" className="px-3.5 py-3 font-bold">
                    {exercise}
                  </TableHead>
                  <TableCell className="px-3.5 py-3 tabular-nums">{recent}</TableCell>
                  <TableCell className="px-3.5 py-3 tabular-nums">{allTime}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      </section>
    </main>
  );
}

function Feature({ title, children }: { title: string; children: ReactNode }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="font-bold">
          <h2>{title}</h2>
        </CardTitle>
        <CardDescription className="text-base">{children}</CardDescription>
      </CardHeader>
    </Card>
  );
}

/** Sign in as any login without OAuth. Only in `make dev`, see BODYWEIGHT_DEV_LOGIN. */
function DevSignIn({ onSignIn }: { onSignIn: (user: User) => void }) {
  const [login, setLogin] = useState("");
  const [error, setError] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    try {
      onSignIn(await devSignIn(login));
    } catch {
      setError(true);
    }
  }

  return (
    <form className="mt-4 flex items-center gap-2 text-sm" onSubmit={handleSubmit}>
      <Input
        className="h-9 max-w-45"
        value={login}
        onChange={(e) => setLogin(e.target.value)}
        placeholder="login"
        aria-label="Dev login"
        autoCapitalize="none"
        required
      />
      <Button type="submit" variant="outline" size="lg">
        Dev sign in
      </Button>
      {error && <span role="alert">Failed</span>}
    </form>
  );
}
