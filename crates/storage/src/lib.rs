//! YDB storage shared by the server and the CLI.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bodyweight_domain::{
    Bodyweight, CustomExercise, ExerciseEntry, ExerciseId, ExerciseName, ExerciseNameError, Reps,
    Set, User, WeighIn, Weight, Workout, WorkoutId,
};
use chrono::NaiveDate;
use ydb::{Client, ClientBuilder, FromEnvCredentials, ResultSet, TxMode, Value};

const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("environment variable {0} is not set")]
    MissingEnv(&'static str),
    #[error(transparent)]
    Ydb(#[from] ydb::YdbError),
    #[error("date {0} is not supported, YDB dates start at 1970-01-01")]
    UnsupportedDate(NaiveDate),
    #[error("corrupt row in {table}: {reason}")]
    Corrupt { table: &'static str, reason: String },
}

pub type Result<T> = std::result::Result<T, StorageError>;

pub struct Storage {
    client: Client,
}

/// What `Storage::delete_custom_exercise` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteExercise {
    Deleted,
    /// Nothing was deleted: the exercise is used in a workout.
    InUse,
    NotFound,
}

/// A signed-in user's session, see `Storage::find_session`.
#[derive(Debug, Clone)]
pub struct Session {
    pub user: User,
    pub expires_at: SystemTime,
}

impl Storage {
    /// Connects using the standard YDB SDK env variables: `YDB_ENDPOINT`, `YDB_DATABASE`
    /// and one of the credential variables (`YDB_ANONYMOUS_CREDENTIALS=1`,
    /// `YDB_SERVICE_ACCOUNT_KEY_FILE_CREDENTIALS`, `YDB_ACCESS_TOKEN_CREDENTIALS`, ...).
    pub async fn connect_from_env() -> Result<Self> {
        let endpoint = env("YDB_ENDPOINT")?;
        let database = env("YDB_DATABASE")?;
        let client =
            ClientBuilder::new_from_connection_string(format!("{endpoint}?database={database}"))?
                .with_credentials(FromEnvCredentials::new()?)
                .build()
                .await?;
        Ok(Self { client })
    }

    /// All weigh-ins of the user, newest first.
    pub async fn list_weigh_ins(&self, user_login: &str) -> Result<Vec<WeighIn>> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(
                "DECLARE $user_login AS Utf8;
                 SELECT date, bodyweight_tenths FROM weigh_ins
                 WHERE user_login = $user_login
                 ORDER BY date DESC;",
            )
            .param("$user_login", user_login)
            .idempotent(true)
            .await?;

        result_set
            .rows()
            .map(|mut row| {
                let date: SystemTime = row.remove_field_by_name("date")?.try_into()?;
                let tenths: u32 = row.remove_field_by_name("bodyweight_tenths")?.try_into()?;
                Ok(WeighIn {
                    date: date_from_ydb(date, "weigh_ins")?,
                    bodyweight: Bodyweight::from_tenths(tenths).map_err(|e| {
                        StorageError::Corrupt {
                            table: "weigh_ins",
                            reason: e.to_string(),
                        }
                    })?,
                })
            })
            .collect()
    }

    /// Saves the weigh-in, replacing an existing one on the same date.
    pub async fn put_weigh_in(&self, user_login: &str, weigh_in: WeighIn) -> Result<()> {
        self.client
            .query_client()
            .exec(
                "DECLARE $user_login AS Utf8;
                 DECLARE $date AS Date;
                 DECLARE $bodyweight_tenths AS Uint32;
                 UPSERT INTO weigh_ins (user_login, date, bodyweight_tenths)
                 VALUES ($user_login, $date, $bodyweight_tenths);",
            )
            .param("$user_login", user_login)
            .param("$date", date_to_ydb(weigh_in.date)?)
            .param("$bodyweight_tenths", weigh_in.bodyweight.tenths())
            .idempotent(true)
            .await?;
        Ok(())
    }

    /// All custom exercises of the user, archived ones included, ordered by name.
    pub async fn list_custom_exercises(&self, user_login: &str) -> Result<Vec<CustomExercise>> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(
                "DECLARE $user_login AS Utf8;
                 SELECT id, name, archived FROM custom_exercises
                 WHERE user_login = $user_login;",
            )
            .param("$user_login", user_login)
            .idempotent(true)
            .await?;

        let mut exercises = result_set
            .rows()
            .map(|mut row| {
                let name: String = row.remove_field_by_name("name")?.try_into()?;
                Ok(CustomExercise {
                    id: row.remove_field_by_name("id")?.try_into()?,
                    name: name
                        .parse()
                        .map_err(|e: ExerciseNameError| StorageError::Corrupt {
                            table: "custom_exercises",
                            reason: e.to_string(),
                        })?,
                    archived: row.remove_field_by_name("archived")?.try_into()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        exercises.sort_by_cached_key(|e| (e.name.as_str().to_lowercase(), e.id.clone()));
        Ok(exercises)
    }

    /// Saves a new custom exercise. The caller picks an `id` that is unique for the user.
    pub async fn create_custom_exercise(
        &self,
        user_login: &str,
        exercise: &CustomExercise,
    ) -> Result<()> {
        self.client
            .query_client()
            .exec(
                "DECLARE $user_login AS Utf8;
                 DECLARE $id AS Utf8;
                 DECLARE $name AS Utf8;
                 DECLARE $archived AS Bool;
                 UPSERT INTO custom_exercises (user_login, id, name, archived)
                 VALUES ($user_login, $id, $name, $archived);",
            )
            .param("$user_login", user_login)
            .param("$id", exercise.id.as_str())
            .param("$name", exercise.name.as_str())
            .param("$archived", exercise.archived)
            .idempotent(true)
            .await?;
        Ok(())
    }

    /// Renames and/or (un)archives a custom exercise; `None` leaves the field as it is.
    /// Returns false if the user has no such exercise.
    pub async fn update_custom_exercise(
        &self,
        user_login: &str,
        id: &str,
        name: Option<&ExerciseName>,
        archived: Option<bool>,
    ) -> Result<bool> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(
                "DECLARE $user_login AS Utf8;
                 DECLARE $id AS Utf8;
                 DECLARE $name AS Utf8?;
                 DECLARE $archived AS Bool?;
                 UPDATE custom_exercises
                 SET name = $name ?? name, archived = $archived ?? archived
                 WHERE user_login = $user_login AND id = $id
                 RETURNING id;",
            )
            .param("$user_login", user_login)
            .param("$id", id)
            .param("$name", name.map(|n| n.as_str().to_owned()))
            .param("$archived", archived)
            .idempotent(true)
            .await?;
        Ok(result_set.rows().next().is_some())
    }

    /// Deletes a custom exercise, unless it is used in a workout.
    pub async fn delete_custom_exercise(
        &self,
        user_login: &str,
        id: &str,
    ) -> Result<DeleteExercise> {
        // The check and the delete are one query, so a workout cannot start using the
        // exercise in between.
        let result_set = self
            .client
            .query_client()
            .query_result_set(
                "DECLARE $user_login AS Utf8;
                 DECLARE $id AS Utf8;
                 $uses = (
                     SELECT COUNT(*) FROM exercise_entries
                     WHERE user_login = $user_login AND exercise_id = $id
                 );
                 DELETE FROM custom_exercises
                 WHERE user_login = $user_login AND id = $id AND $uses = 0
                 RETURNING id;",
            )
            .param("$user_login", user_login)
            .param("$id", id)
            .with_tx_mode(TxMode::SerializableReadWrite)
            .await?;
        if result_set.rows().next().is_some() {
            return Ok(DeleteExercise::Deleted);
        }
        let exists = self
            .list_custom_exercises(user_login)
            .await?
            .iter()
            .any(|e| e.id == id);
        Ok(if exists {
            DeleteExercise::InUse
        } else {
            DeleteExercise::NotFound
        })
    }

    /// All workouts of the user with their exercise entries and sets, newest first:
    /// by date, then by creation time.
    pub async fn list_workouts(&self, user_login: &str) -> Result<Vec<Workout>> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(workout_rows_query(false))
            .param("$user_login", user_login)
            .idempotent(true)
            .await?;
        workouts_from_rows(result_set)
    }

    pub async fn get_workout(&self, user_login: &str, id: &WorkoutId) -> Result<Option<Workout>> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(workout_rows_query(true))
            .param("$user_login", user_login)
            .param("$id", id.as_str())
            .idempotent(true)
            .await?;
        Ok(workouts_from_rows(result_set)?.pop())
    }

    /// Creates the workout or replaces the one with the same id, including its exercise
    /// entries and sets. `now` becomes the creation time of a new workout.
    pub async fn put_workout(
        &self,
        user_login: &str,
        workout: &Workout,
        now: SystemTime,
    ) -> Result<()> {
        let field = |name: &str, value: Value| (name.to_owned(), value);
        let position = |index: usize| Value::Uint32(u32::try_from(index).expect("few items"));

        let mut entries = Vec::new();
        let mut sets = Vec::new();
        for (entry_index, entry) in workout.entries.iter().enumerate() {
            entries.push(Value::struct_from_fields(vec![
                field("position", position(entry_index)),
                field("exercise_id", entry.exercise.as_str().into()),
            ]));
            for (set_index, set) in entry.sets.iter().enumerate() {
                sets.push(Value::struct_from_fields(vec![
                    field("entry_position", position(entry_index)),
                    field("position", position(set_index)),
                    field("reps", set.reps.get().into()),
                    field("weight_quarters", set.weight.map(Weight::quarters).into()),
                ]));
            }
        }
        // The item type of the lists, needed when they are empty.
        let entry_type = Value::struct_from_fields(vec![
            field("position", 0u32.into()),
            field("exercise_id", "".into()),
        ]);
        let set_type = Value::struct_from_fields(vec![
            field("entry_position", 0u32.into()),
            field("position", 0u32.into()),
            field("reps", 0u32.into()),
            field("weight_quarters", None::<u32>.into()),
        ]);

        self.client
            .query_client()
            .exec(
                "DECLARE $user_login AS Utf8;
                 DECLARE $id AS Utf8;
                 DECLARE $date AS Date;
                 DECLARE $note AS Utf8;
                 DECLARE $now AS Timestamp;
                 DECLARE $entries AS List<Struct<position: Uint32, exercise_id: Utf8>>;
                 DECLARE $sets AS List<Struct<
                     entry_position: Uint32, position: Uint32, reps: Uint32, weight_quarters: Uint32?
                 >>;

                 $created_at = (
                     SELECT created_at FROM workouts WHERE user_login = $user_login AND id = $id
                 );
                 UPSERT INTO workouts (user_login, id, date, note, created_at)
                 VALUES ($user_login, $id, $date, $note, $created_at ?? $now);

                 DELETE FROM exercise_entries WHERE user_login = $user_login AND workout_id = $id;
                 DELETE FROM sets WHERE user_login = $user_login AND workout_id = $id;

                 UPSERT INTO exercise_entries
                 SELECT $user_login AS user_login, $id AS workout_id, position, exercise_id
                 FROM AS_TABLE($entries);
                 UPSERT INTO sets
                 SELECT $user_login AS user_login, $id AS workout_id,
                        entry_position, position, reps, weight_quarters
                 FROM AS_TABLE($sets);",
            )
            .param("$user_login", user_login)
            .param("$id", workout.id.as_str())
            .param("$date", date_to_ydb(workout.date)?)
            .param("$note", workout.note.as_str())
            .param("$now", Value::Timestamp(now))
            .param("$entries", Value::list_from(entry_type, entries)?)
            .param("$sets", Value::list_from(set_type, sets)?)
            .with_tx_mode(TxMode::SerializableReadWrite)
            .idempotent(true)
            .await?;
        Ok(())
    }

    /// Deletes a workout with its exercise entries and sets. Returns false if the user has
    /// no such workout.
    pub async fn delete_workout(&self, user_login: &str, id: &WorkoutId) -> Result<bool> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(
                "DECLARE $user_login AS Utf8;
                 DECLARE $id AS Utf8;
                 DELETE FROM sets WHERE user_login = $user_login AND workout_id = $id;
                 DELETE FROM exercise_entries WHERE user_login = $user_login AND workout_id = $id;
                 DELETE FROM workouts WHERE user_login = $user_login AND id = $id
                 RETURNING id;",
            )
            .param("$user_login", user_login)
            .param("$id", id.as_str())
            .with_tx_mode(TxMode::SerializableReadWrite)
            .await?;
        Ok(result_set.rows().next().is_some())
    }

    /// Creates the user or updates their profile. `yandex_uid` is absent for dev-login users.
    pub async fn upsert_user(&self, user: &User, yandex_uid: Option<&str>) -> Result<()> {
        self.client
            .query_client()
            .exec(
                "DECLARE $login AS Utf8;
                 DECLARE $yandex_uid AS Utf8?;
                 DECLARE $display_name AS Utf8;
                 DECLARE $avatar_id AS Utf8?;
                 UPSERT INTO users (login, yandex_uid, display_name, avatar_id)
                 VALUES ($login, $yandex_uid, $display_name, $avatar_id);",
            )
            .param("$login", user.login.as_str())
            .param("$yandex_uid", yandex_uid.map(str::to_owned))
            .param("$display_name", user.display_name.as_str())
            .param("$avatar_id", user.avatar_id.clone())
            .idempotent(true)
            .await?;
        Ok(())
    }

    /// Creates a session. `id` is whatever the caller looks sessions up by;
    /// the server passes a hash of the cookie value.
    pub async fn create_session(
        &self,
        id: &str,
        user_login: &str,
        expires_at: SystemTime,
    ) -> Result<()> {
        self.client
            .query_client()
            .exec(
                "DECLARE $id AS Utf8;
                 DECLARE $user_login AS Utf8;
                 DECLARE $expires_at AS Timestamp;
                 UPSERT INTO sessions (id, user_login, expires_at)
                 VALUES ($id, $user_login, $expires_at);",
            )
            .param("$id", id)
            .param("$user_login", user_login)
            .param("$expires_at", Value::Timestamp(expires_at))
            .idempotent(true)
            .await?;
        Ok(())
    }

    /// The session and its user, unless the session is missing or expired at `now`.
    pub async fn find_session(&self, id: &str, now: SystemTime) -> Result<Option<Session>> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(
                "DECLARE $id AS Utf8;
                 DECLARE $now AS Timestamp;
                 SELECT u.login AS login, u.display_name AS display_name,
                        u.avatar_id AS avatar_id, s.expires_at AS expires_at
                 FROM sessions AS s
                 INNER JOIN users AS u ON u.login = s.user_login
                 WHERE s.id = $id AND s.expires_at > $now;",
            )
            .param("$id", id)
            .param("$now", Value::Timestamp(now))
            .idempotent(true)
            .await?;

        result_set
            .rows()
            .next()
            .map(|mut row| {
                Ok(Session {
                    user: User {
                        login: row.remove_field_by_name("login")?.try_into()?,
                        display_name: row.remove_field_by_name("display_name")?.try_into()?,
                        avatar_id: row.remove_field_by_name("avatar_id")?.try_into()?,
                    },
                    expires_at: row.remove_field_by_name("expires_at")?.try_into()?,
                })
            })
            .transpose()
    }

    /// Moves the expiry of an existing session; does nothing if the session is gone.
    pub async fn extend_session(&self, id: &str, expires_at: SystemTime) -> Result<()> {
        self.client
            .query_client()
            .exec(
                "DECLARE $id AS Utf8;
                 DECLARE $expires_at AS Timestamp;
                 UPDATE sessions SET expires_at = $expires_at WHERE id = $id;",
            )
            .param("$id", id)
            .param("$expires_at", Value::Timestamp(expires_at))
            .idempotent(true)
            .await?;
        Ok(())
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        self.client
            .query_client()
            .exec(
                "DECLARE $id AS Utf8;
                 DELETE FROM sessions WHERE id = $id;",
            )
            .param("$id", id)
            .idempotent(true)
            .await?;
        Ok(())
    }
}

/// Selects workouts joined with their exercise entries and sets: a row per set, or a single
/// row with NULLs for a workout without entries or an entry without sets.
fn workout_rows_query(by_id: bool) -> String {
    let (declare_id, filter) = if by_id {
        ("DECLARE $id AS Utf8;", "AND w.id = $id")
    } else {
        ("", "")
    };
    format!(
        "DECLARE $user_login AS Utf8;
         {declare_id}
         SELECT w.id AS id, w.date AS date, w.note AS note, w.created_at AS created_at,
                e.position AS entry_position, e.exercise_id AS exercise_id,
                s.position AS set_position, s.reps AS reps, s.weight_quarters AS weight_quarters
         FROM workouts AS w
         LEFT JOIN exercise_entries AS e
             ON e.user_login = w.user_login AND e.workout_id = w.id
         LEFT JOIN sets AS s
             ON s.user_login = e.user_login AND s.workout_id = e.workout_id
                AND s.entry_position = e.position
         WHERE w.user_login = $user_login {filter}
         ORDER BY date DESC, created_at DESC, id, entry_position, set_position;"
    )
}

/// Groups the rows of `workout_rows_query` back into workouts, keeping their order.
fn workouts_from_rows(result_set: ResultSet) -> Result<Vec<Workout>> {
    fn corrupt(table: &'static str, error: impl ToString) -> StorageError {
        StorageError::Corrupt {
            table,
            reason: error.to_string(),
        }
    }

    let mut workouts: Vec<Workout> = Vec::new();
    // The position of the last entry of the last workout.
    let mut last_entry = None;
    for mut row in result_set.rows() {
        let id: String = row.remove_field_by_name("id")?.try_into()?;
        if workouts.last().is_none_or(|w| w.id.as_str() != id) {
            let date: SystemTime = row.remove_field_by_name("date")?.try_into()?;
            workouts.push(Workout {
                id: id.parse().map_err(|e| corrupt("workouts", e))?,
                date: date_from_ydb(date, "workouts")?,
                note: row.remove_field_by_name("note")?.try_into()?,
                entries: Vec::new(),
            });
            last_entry = None;
        }
        let workout = workouts.last_mut().expect("just pushed");

        let entry_position: Option<u32> = row.remove_field_by_name("entry_position")?.try_into()?;
        if entry_position.is_none() {
            continue;
        }
        if entry_position != last_entry {
            let exercise_id: Option<String> =
                row.remove_field_by_name("exercise_id")?.try_into()?;
            workout.entries.push(ExerciseEntry {
                exercise: ExerciseId::parse(&exercise_id.unwrap_or_default()),
                sets: Vec::new(),
            });
            last_entry = entry_position;
        }

        let reps: Option<u32> = row.remove_field_by_name("reps")?.try_into()?;
        let Some(reps) = reps else { continue };
        let weight: Option<u32> = row.remove_field_by_name("weight_quarters")?.try_into()?;
        let entry = workout.entries.last_mut().expect("just pushed");
        entry.sets.push(Set {
            reps: Reps::new(reps).map_err(|e| corrupt("sets", e))?,
            weight: weight
                .map(Weight::from_quarters)
                .transpose()
                .map_err(|e| corrupt("sets", e))?,
        });
    }
    Ok(workouts)
}

fn env(name: &'static str) -> Result<String> {
    std::env::var(name).map_err(|_| StorageError::MissingEnv(name))
}

fn epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date")
}

fn date_to_ydb(date: NaiveDate) -> Result<Value> {
    let days = date.signed_duration_since(epoch()).num_days();
    let days = u64::try_from(days).map_err(|_| StorageError::UnsupportedDate(date))?;
    Ok(Value::Date(
        UNIX_EPOCH + Duration::from_secs(days * SECONDS_PER_DAY),
    ))
}

fn date_from_ydb(time: SystemTime, table: &'static str) -> Result<NaiveDate> {
    let corrupt = |reason: String| StorageError::Corrupt { table, reason };
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map_err(|e| corrupt(e.to_string()))?
        .as_secs();
    epoch()
        .checked_add_days(chrono::Days::new(secs / SECONDS_PER_DAY))
        .ok_or_else(|| corrupt(format!("date out of range: {secs}s since epoch")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_round_trips() {
        for date in [
            epoch(),
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
            NaiveDate::from_ymd_opt(2000, 2, 29).unwrap(),
        ] {
            let Value::Date(time) = date_to_ydb(date).unwrap() else {
                unreachable!()
            };
            assert_eq!(date_from_ydb(time, "weigh_ins").unwrap(), date);
        }
    }

    #[test]
    fn rejects_dates_before_epoch() {
        let date = NaiveDate::from_ymd_opt(1969, 12, 31).unwrap();
        assert!(matches!(
            date_to_ydb(date),
            Err(StorageError::UnsupportedDate(d)) if d == date
        ));
    }
}
