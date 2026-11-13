# TODO

What is left to implement from [docs/spec.md](docs/spec.md), in the intended order.

## 1. Exercises (§1.4)

- [x] Catalog of standard exercises in code: pull-ups and the Big Three
- [x] Custom exercises in the domain: create, rename, archive, delete
- [x] Migration and storage for custom exercises
- [x] Exercises API
- [x] My exercises screen

## 2. Workouts (§1.1–1.3)

- [ ] Domain model: workout, exercise entry, set (reps ≥ 1, weight in steps of 0.25, no future dates)
- [ ] Migrations and storage for workouts, exercise entries and sets
- [ ] Workouts API
- [ ] Refuse to delete a custom exercise that is used in a workout (§1.4)
- [ ] Workouts screen: list, newest first, with date, exercises and the start of the note
- [ ] Workout screen: view and edit date, note, exercise entries and sets

## 3. Adding an exercise and saving (§4.2–4.3)

- [ ] Suggested sets from the last exercise entry of the same exercise, with a "done" mark
- [ ] Only marked sets are saved; one empty set row if the exercise was never done
- [ ] "Repeat set" button
- [ ] Autosave: the workout is created at once, every change goes to the server
- [ ] Offline queue in localStorage, "not saved" status and retries

## 4. Bodyweight ratio (§1.6)

- [ ] Calculation in the domain: rounded down to tenths, bodyweight from the latest weigh-in on or before the workout date
- [ ] Shown on sets with a weight on the Workout screen

## 5. PRs (§2)

- [ ] Estimated 1RM by the Epley formula for the Big Three (sets with a weight and at most 12 reps)
- [ ] Max reps for pull-ups (sets without a weight)
- [ ] Periods: 90 days and all time, "today" in Europe/Moscow; ties go to the earliest set
- [ ] PRs API
- [ ] PRs screen: standard exercises × {90 days, All time}

## 6. CLI (§5)

- [ ] Create a workout, add an exercise entry and sets
- [ ] List workouts
- [x] List exercises
- [ ] Show current PRs

## 7. Tests (§7.1)

- [ ] Unit tests: PRs, estimated 1RM, bodyweight ratio, the 90-day window
- [ ] Integration tests for the new storage and API

## Gaps in what is already done

- [ ] `make lint`: add eslint and prettier for the frontend (§7)
