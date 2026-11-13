import { useCallback, useEffect, useState } from "react";
import type { FormEvent } from "react";
import { ApiError, UnauthorizedError } from "./api";
import { addDays, toIsoDate } from "./dates";
import { fetchWeighIns, MAX_DAYS_AHEAD, normalizeBodyweight, putWeighIn } from "./weighIns";
import type { WeighIn } from "./weighIns";

/** The Weigh-ins screen: add a weigh-in and see the list, newest first. */
export function WeighInsScreen({ onUnauthorized }: { onUnauthorized: () => void }) {
  // null: not loaded yet.
  const [weighIns, setWeighIns] = useState<WeighIn[] | null>(null);
  const [date, setDate] = useState(() => toIsoDate(new Date()));
  const [bodyweight, setBodyweight] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fail = useCallback(
    (e: unknown, fallback: string) => {
      if (e instanceof UnauthorizedError) onUnauthorized();
      else setError(e instanceof ApiError && e.status === 400 ? capitalize(e.message) : fallback);
    },
    [onUnauthorized],
  );

  useEffect(() => {
    fetchWeighIns().then(setWeighIns, (e: unknown) => fail(e, "Could not load weigh-ins."));
  }, [fail]);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await putWeighIn(date, normalizeBodyweight(bodyweight));
      setBodyweight("");
      setWeighIns(await fetchWeighIns());
    } catch (e) {
      fail(e, "Could not save the weigh-in. Please try again.");
    } finally {
      setSaving(false);
    }
  }

  const existing = weighIns?.find((w) => w.date === date);

  return (
    <section className="weigh-ins">
      <h1 className="screen-title">Weigh-ins</h1>

      <form className="weigh-in-form" onSubmit={handleSubmit}>
        <label>
          <span>Date</span>
          <input
            type="date"
            value={date}
            max={toIsoDate(addDays(new Date(), MAX_DAYS_AHEAD))}
            onChange={(e) => setDate(e.target.value)}
            required
          />
        </label>
        <label>
          <span>Bodyweight, kg</span>
          <input
            value={bodyweight}
            onChange={(e) => setBodyweight(e.target.value)}
            inputMode="decimal"
            autoComplete="off"
            placeholder={weighIns?.[0]?.bodyweight ?? "82.5"}
            required
          />
        </label>
        <button className="primary" type="submit" disabled={saving}>
          {saving ? "Saving…" : "Add"}
        </button>
      </form>
      {existing && (
        <p className="hint">
          Replaces {existing.bodyweight} kg on {existing.date}.
        </p>
      )}
      {error && (
        <p className="hint error" role="alert">
          {error}
        </p>
      )}

      {weighIns?.length === 0 && <p className="empty">No weigh-ins yet.</p>}
      {weighIns && weighIns.length > 0 && (
        <table className="weigh-in-list">
          <thead>
            <tr>
              <th>Date</th>
              <th className="number">kg</th>
            </tr>
          </thead>
          <tbody>
            {weighIns.map((w) => (
              <tr key={w.date}>
                <td>{w.date}</td>
                <td className="number">{w.bodyweight}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function capitalize(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}
