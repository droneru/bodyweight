import type { ReactNode } from "react";

/** Landing page: explains what the service is. The app screens come later. */
export function App() {
  return (
    <div className="page">
      <header className="top">
        <span className="logo">Bodyweight</span>
      </header>

      <main>
        <section className="hero">
          <h1>A simple strength-training log</h1>
          <p className="lead">
            Record your workouts and weigh-ins. See your PRs in pull-ups and the Big Three for the
            last 90 days and all time.
          </p>
          <button className="cta" type="button" disabled>
            Sign in with Yandex ID
          </button>
          <p className="soon">Sign-in is coming soon.</p>
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

      <footer className="bottom">bodyweight.ru</footer>
    </div>
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
