//! YDB storage shared by the server and the CLI.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bodyweight_domain::{Bodyweight, WeighIn};
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
