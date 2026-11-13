import { useCallback, useEffect, useState } from "react";
import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { errorMessage, UnauthorizedError } from "./api";
import { addDays, toIsoDate } from "./dates";
import { fetchWeighIns, MAX_DAYS_AHEAD, normalizeBodyweight, putWeighIn } from "./weighIns";
import type { WeighIn } from "./weighIns";

/** Form fields tall enough for a thumb. */
const FIELD = "h-11 px-3 tabular-nums";

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
      else setError(errorMessage(e, fallback));
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
    <section>
      <h1 className="mt-6 mb-4 text-2xl font-bold">Weigh-ins</h1>

      <form className="grid grid-cols-2 gap-3" onSubmit={handleSubmit}>
        <div className="grid min-w-0 gap-1.5">
          <Label htmlFor="weigh-in-date" className="font-normal text-muted-foreground">
            Date
          </Label>
          <Input
            id="weigh-in-date"
            className={FIELD}
            type="date"
            value={date}
            max={toIsoDate(addDays(new Date(), MAX_DAYS_AHEAD))}
            onChange={(e) => setDate(e.target.value)}
            required
          />
        </div>
        <div className="grid min-w-0 gap-1.5">
          <Label htmlFor="weigh-in-bodyweight" className="font-normal text-muted-foreground">
            Bodyweight, kg
          </Label>
          <Input
            id="weigh-in-bodyweight"
            className={FIELD}
            value={bodyweight}
            onChange={(e) => setBodyweight(e.target.value)}
            inputMode="decimal"
            autoComplete="off"
            placeholder={weighIns?.[0]?.bodyweight ?? "82.5"}
            required
          />
        </div>
        <Button className="col-span-2 h-11 text-base font-semibold" type="submit" disabled={saving}>
          {saving ? "Saving…" : "Add"}
        </Button>
      </form>
      {existing && (
        <p className="mt-2.5 text-sm text-muted-foreground">
          Replaces {existing.bodyweight} kg on {existing.date}.
        </p>
      )}
      {error && (
        <p className="mt-2.5 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}

      {weighIns?.length === 0 && <p className="my-6 text-muted-foreground">No weigh-ins yet.</p>}
      {weighIns && weighIns.length > 0 && (
        <Card className="mt-6 py-0">
          <Table className="text-base">
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead className="px-3.5 text-muted-foreground">Date</TableHead>
                <TableHead className="px-3.5 text-right text-muted-foreground">kg</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {weighIns.map((w) => (
                <TableRow key={w.date} className="hover:bg-transparent">
                  <TableCell className="px-3.5 py-3 tabular-nums">{w.date}</TableCell>
                  <TableCell className="px-3.5 py-3 text-right tabular-nums">
                    {w.bodyweight}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}
    </section>
  );
}
