export type SaveStatus = "saved" | "saving" | "failed";

/**
 * Keeps a value saved: after `set`, `flush` saves the latest value unless it is the one
 * already saved. Values are compared as JSON. One save runs at a time; a value set during
 * a save is saved next.
 */
export class Autosaver<T> {
  private savedJson: string;
  private latest: T | null = null;
  private running: Promise<void> | null = null;
  private paused = false;

  /**
   * `initial` is what is already saved. `onStatus` is told when a save starts, and how a
   * flush ended; `error` is what a failed save threw.
   */
  constructor(
    initial: T,
    private readonly save: (value: T) => Promise<void>,
    private readonly onStatus: (status: SaveStatus, error: unknown) => void,
  ) {
    this.savedJson = JSON.stringify(initial);
  }

  /**
   * Sets the value to save on the next flush; null stands for a value that cannot be saved.
   * Returns whether there is something to save now.
   */
  set(value: T | null): boolean {
    this.latest = value;
    return this.isDirty();
  }

  /** Saves until the latest value is the saved one. Never rejects: see `onStatus`. */
  flush(): Promise<void> {
    if (this.running) return this.running;
    // The callback of finally() runs after this method returns, even if there is nothing to save.
    const run = this.saveAll().finally(() => {
      this.running = null;
    });
    this.running = run;
    return run;
  }

  /** Stops saving and waits for the save under way, e.g. before the value is deleted. */
  async pause(): Promise<void> {
    this.paused = true;
    await this.running;
  }

  resume(): void {
    this.paused = false;
    void this.flush();
  }

  private isDirty(): boolean {
    return this.latest !== null && JSON.stringify(this.latest) !== this.savedJson;
  }

  private async saveAll(): Promise<void> {
    try {
      while (!this.paused && this.isDirty()) {
        const value = this.latest as T;
        const json = JSON.stringify(value);
        this.onStatus("saving", null);
        await this.save(value);
        this.savedJson = json;
      }
      if (!this.isDirty()) this.onStatus("saved", null);
    } catch (e) {
      this.onStatus("failed", e);
    }
  }
}
