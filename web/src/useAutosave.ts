import { useCallback, useEffect, useRef, useState } from "react";
import { Autosaver } from "./autosaver";
import type { SaveStatus } from "./autosaver";

/** How long to wait for the next change before saving. */
const DELAY_MS = 500;
/** How long to wait before sending a failed save again. */
const RETRY_MS = 5000;

export interface Autosave {
  /** `saving` also covers the short wait before a change is sent. */
  status: SaveStatus;
  /** What the last failed save threw. */
  error: unknown;
  /** Tries to save again after a failure, without waiting for the next automatic try. */
  retry: () => void;
  /** Stops saving and waits for the save under way, e.g. before the value is deleted. */
  pause: () => Promise<void>;
  resume: () => void;
}

/** Where a value waits until the server has it, so that it survives a closed page. */
export interface UnsavedStore<T> {
  put: (value: T) => void;
  /** Called once `saved` is on the server. */
  clear: (saved: T) => void;
}

export interface AutosaveOptions<T> {
  store?: UnsavedStore<T>;
  /** Whether sending again can help after this failure. By default it always can. */
  canRetry?: (error: unknown) => boolean;
}

/**
 * Saves `value` shortly after every change, and at once when the component goes away.
 * A failed save is repeated every few seconds and when the browser comes back online.
 * `initial` is what is already saved; a null `value` cannot be saved and is skipped.
 */
export function useAutosave<T>(
  value: T | null,
  initial: T,
  save: (value: T) => Promise<void>,
  { store, canRetry }: AutosaveOptions<T> = {},
): Autosave {
  const [state, setState] = useState<{ status: SaveStatus; error: unknown }>({
    status: "saved",
    error: null,
  });
  const saveRef = useRef(save);
  const storeRef = useRef(store);
  const [saver] = useState(
    // oxlint-disable-next-line react/refs -- the refs are read when a save runs, not while rendering
    () =>
      new Autosaver<T>(
        initial,
        async (v) => {
          await saveRef.current(v);
          storeRef.current?.clear(v);
        },
        (status, error) => setState({ status, error }),
      ),
  );

  useEffect(() => {
    saveRef.current = save;
    storeRef.current = store;
  }, [save, store]);

  useEffect(() => {
    if (saver.set(value)) {
      if (value !== null) storeRef.current?.put(value);
    } else if (value !== null) {
      // Back at the saved value: what was put aside is out of date.
      storeRef.current?.clear(value);
    }
    const timer = setTimeout(() => void saver.flush(), DELAY_MS);
    return () => clearTimeout(timer);
  }, [value, saver]);

  useEffect(() => () => void saver.flush(), [saver]);

  const retrying = state.status === "failed" && (canRetry?.(state.error) ?? true);
  useEffect(() => {
    if (!retrying) return;
    const flush = () => void saver.flush();
    const timer = setInterval(flush, RETRY_MS);
    window.addEventListener("online", flush);
    return () => {
      clearInterval(timer);
      window.removeEventListener("online", flush);
    };
  }, [retrying, saver]);

  const retry = useCallback(() => void saver.flush(), [saver]);
  const pause = useCallback(() => saver.pause(), [saver]);
  const resume = useCallback(() => saver.resume(), [saver]);
  return { ...state, retry, pause, resume };
}
