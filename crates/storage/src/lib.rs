//! YDB storage shared by the server and the CLI.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bodyweight_domain::{
    Bodyweight, CustomExercise, ExerciseName, ExerciseNameError, User, WeighIn,
};
use chrono::NaiveDate;
use ydb::{Client, ClientBuilder, FromEnvCredentials, Value};

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
                    date: date_from_ydb(date)?,
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

    /// Deletes a custom exercise. Returns false if the user has no such exercise.
    pub async fn delete_custom_exercise(&self, user_login: &str, id: &str) -> Result<bool> {
        let result_set = self
            .client
            .query_client()
            .query_result_set(
                "DECLARE $user_login AS Utf8;
                 DECLARE $id AS Utf8;
                 DELETE FROM custom_exercises
                 WHERE user_login = $user_login AND id = $id
                 RETURNING id;",
            )
            .param("$user_login", user_login)
            .param("$id", id)
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

fn date_from_ydb(time: SystemTime) -> Result<NaiveDate> {
    let corrupt = |reason: String| StorageError::Corrupt {
        table: "weigh_ins",
        reason,
    };
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
            assert_eq!(date_from_ydb(time).unwrap(), date);
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
