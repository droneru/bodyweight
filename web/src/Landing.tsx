import { useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { devSignIn, YANDEX_SIGN_IN_URL } from "./auth";
import type { User } from "./auth";

/** What signed-out visitors see: explains what the service is and offers to sign in. */
export function Landing({ onSignIn }: { onSignIn: (user: User) => void }) {
  const signInFailed = new URLSearchParams(window.location.search).get("sign_in") === "failed";

  return (
    <main>
      <section className="hero">
        <h1>A simple strength-training log</h1>
        <p className="lead">
          Record your workouts and weigh-ins. See your PRs in pull-ups and the Big Three for the
          last 90 days and all time.
        </p>
        <a className="cta" href={YANDEX_SIGN_IN_URL}>
          Sign in with Yandex ID
        </a>
        {signInFailed && (
          <p className="hint" role="alert">
            Sign-in failed. Please try again.
          </p>
        )}
        {import.meta.env.DEV && <DevSignIn onSignIn={onSignIn} />}
      </section>

      <section className="features">
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

      <section className="example" aria-label="PRs example">
        <h2>PRs at a glance</h2>
        <table>
          <thead>
            <tr>
              <th></th>
              <th>90 days</th>
              <th>All time</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <th>Pull-ups</th>
              <td>15 reps</td>
              <td>18 reps</td>
            </tr>
            <tr>
              <th>Bench press</th>
              <td>100 kg</td>
              <td>110 kg</td>
            </tr>
            <tr>
              <th>Squat</th>
              <td>120 kg</td>
              <td>140 kg</td>
            </tr>
            <tr>
              <th>Deadlift</th>
              <td>—</td>
              <td>180 kg</td>
            </tr>
          </tbody>
        </table>
      </section>
    </main>
  );
}

function Feature({ title, children }: { title: string; children: ReactNode }) {
  return (
    <article className="feature">
      <h2>{title}</h2>
      <p>{children}</p>
    </article>
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
    <form className="dev-sign-in" onSubmit={handleSubmit}>
      <input
        value={login}
        onChange={(e) => setLogin(e.target.value)}
        placeholder="login"
        aria-label="Dev login"
        autoCapitalize="none"
        required
      />
      <button type="submit">Dev sign in</button>
      {error && <span role="alert">Failed</span>}
    </form>
  );
}
