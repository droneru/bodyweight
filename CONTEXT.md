# Bodyweight

A strength-training log: a user records workouts and weigh-ins, and the service shows their PRs in the standard exercises.

## Language

### Workouts

**Workout** (Тренировка):
A user's training session on a specific date (no time). Holds an ordered list of exercise entries and an optional note. A user may have several workouts on the same date.
_Avoid_: Session, training

**Exercise Entry** (Выполнение упражнения):
One exercise performed within a workout, together with its sets. Entries within a workout are ordered; the same exercise may appear in a workout more than once.
_Avoid_: Workout exercise, block

**Set** (Подход):
A single performance of an exercise: a rep count (integer ≥ 1) and, optionally, a weight. Sets within an entry are ordered, and the order matters.
_Avoid_: Series

**Reps** (Повторы):
How many times the movement was performed in one set.
_Avoid_: Repetitions, times

**Weight** (Вес снаряда):
The external load of a set in kilograms, in steps of 0.25. If there is no weight, the set was done with bodyweight only.
_Avoid_: Load, resistance

**Suggested Set** (Предложенный подход):
A set copied from the last entry of the same exercise when an exercise is added to a workout. The user can mark it done or edit it. An unmarked suggested set is never saved and is not a set.
_Avoid_: Template, plan, draft

**Note** (Заметка):
Free text attached to a workout.
_Avoid_: Comment, description

### Exercises

**Exercise** (Упражнение):
A kind of movement from the catalog, e.g. "bench press".
_Avoid_: Movement, activity

**Standard Exercise** (Стандартное упражнение):
An exercise defined in code, available to every user, that has PRs: pull-ups and the Big Three.
_Avoid_: Core exercise, main exercise, base exercise, built-in exercise

**Big Three** (Большая тройка):
The standard exercises whose PR is the Estimated 1RM: bench press, squat, deadlift.
_Avoid_: Barbell lifts, powerlifts, base exercises

**Custom Exercise** (Пользовательское упражнение):
An exercise a user created for themselves. It is private to that user, and no PRs are computed for it.
_Avoid_: User exercise, own exercise

### PRs

**PR** (Рекорд):
A user's personal record: their best result in a standard exercise over a period, together with the set and date it came from. It is computed on demand and never stored.
_Avoid_: Record, personal best, max, achievement

**Period** (Период):
The window a PR is computed over: the last 90 days (including today), or all time.
_Avoid_: Interval, range

**Estimated 1RM** (Расчётный 1ПМ):
A set's one-rep-max estimate by the Epley formula `w × (1 + r/30)`. It is the PR measure for the Big Three: the maximum over the period's sets that have a weight and at most 12 reps.
_Avoid_: e1RM, one-rep max (unqualified)

**Max Reps** (Рекорд повторов):
The highest rep count in a single set without weight. It is the PR measure for pull-ups.
_Avoid_: Max, rep max

### Bodyweight

**Weigh-in** (Взвешивание):
The user's bodyweight on a date, with 0.1 kg precision. There is at most one weigh-in per date.
_Avoid_: Measurement, weight

**Bodyweight Ratio** (Доля от веса тела):
A set's weight divided by the bodyweight from the latest weigh-in on or before the workout date, rounded down to tenths (e.g. 1.2). It is shown only on sets within a workout, never on PRs.
_Avoid_: Relative weight, coefficient

### Actors

**User** (Пользователь):
A person who signed in with Yandex ID and keeps their own workouts. Identified by their Yandex login.
_Avoid_: Client, account, athlete

**Admin** (Администратор):
An operator with direct database access who works through the CLI on behalf of a user.
_Avoid_: Operator, superuser
