import { useCallback, useEffect, useRef, useState } from "react";
import { Autosaver } from "./autosaver";
import type { SaveStatus } from "./autosaver";

/** How long to wait for the next change before saving. */
const DELAY_MS = 500;

export interface Autosave {
  /** `saving` also covers the short wait before a change is sent. */
  status: SaveStatus;
  /** What the last failed save threw. */
  error: unknown;
  /** Tries to save again after a failure. */
  retry: () => void;
  /** Stops saving and waits for the save under way, e.g. before the value is deleted. */
  pause: () => Promise<void>;
  resume: () => void;
}

/**
 * Saves `value` shortly after every change, and at once when the component goes away.
 * `initial` is what is already saved; a null `value` cannot be saved and is skipped.
 */
export function useAutosave<T>(
  value: T | null,
  initial: T,
  save: (value: T) => Promise<void>,
): Autosave {
  const [state, setState] = useState<{ status: SaveStatus; error: unknown }>({
    status: "saved",
    error: null,
  });
  const saveRef = useRef(save);
  const [saver] = useState(
    () =>
      new Autosaver<T>(
        initial,
        (v) => saveRef.current(v),
        (status, error) => setState({ status, error }),
      ),
  );

  useEffect(() => {
    saveRef.current = save;
  }, [save]);

  useEffect(() => {
    if (saver.set(value)) setState({ status: "saving", error: null });
    const timer = setTimeout(() => void saver.flush(), DELAY_MS);
    return () => clearTimeout(timer);
  }, [value, saver]);

  useEffect(() => () => void saver.flush(), [saver]);

  const retry = useCallback(() => void saver.flush(), [saver]);
  const pause = useCallback(() => saver.pause(), [saver]);
  const resume = useCallback(() => saver.resume(), [saver]);
  return { ...state, retry, pause, resume };
}
