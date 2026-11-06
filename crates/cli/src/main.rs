//! Admin CLI: works directly with YDB on behalf of a user.

// The ydb SDK futures are deeply nested, see https://github.com/rust-lang/rust/issues/152942
#![recursion_limit = "256"]

use anyhow::Context;
use bodyweight_domain::{Bodyweight, WeighIn};
use bodyweight_storage::Storage;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};

/// "Today" is counted in this time zone, see docs/spec.md.
const TIME_ZONE: chrono_tz::Tz = chrono_tz::Europe::Moscow;

/// Bodyweight admin CLI. Connects to YDB using YDB_ENDPOINT, YDB_DATABASE and
/// the standard YDB SDK credential variables.
#[derive(Parser)]
#[command(name = "bodyweight", version)]
struct Cli {
    /// Yandex login of the user to act on behalf of.
    #[arg(long, env = "BODYWEIGHT_USER", value_name = "LOGIN")]
    user: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Weigh-ins: the user's bodyweight by date.
    #[command(subcommand)]
    WeighIns(WeighInsCommand),
}

#[derive(Subcommand)]
enum WeighInsCommand {
    /// List weigh-ins, newest first.
    List,
    /// Add a weigh-in. Replaces an existing weigh-in on the same date.
    Add {
        /// Bodyweight in kg with 0.1 kg precision, e.g. 82.5.
        bodyweight: Bodyweight,
        /// Date in YYYY-MM-DD format. Defaults to today in Europe/Moscow.
        #[arg(long)]
        date: Option<NaiveDate>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let user = cli.user.trim();
    anyhow::ensure!(!user.is_empty(), "BODYWEIGHT_USER must not be empty");

    let storage = Storage::connect_from_env()
        .await
        .context("failed to connect to YDB")?;

    match cli.command {
        Command::WeighIns(WeighInsCommand::List) => {
            let weigh_ins = storage.list_weigh_ins(user).await?;
            if weigh_ins.is_empty() {
                println!("No weigh-ins yet.");
                return Ok(());
            }
            println!("{:<10}  {:>6}", "DATE", "KG");
            for w in weigh_ins {
                println!("{:<10}  {:>6}", w.date, w.bodyweight.to_string());
            }
        }
        Command::WeighIns(WeighInsCommand::Add { bodyweight, date }) => {
            let date =
                date.unwrap_or_else(|| chrono::Utc::now().with_timezone(&TIME_ZONE).date_naive());
            storage
                .put_weigh_in(user, WeighIn { date, bodyweight })
                .await?;
            println!("Saved weigh-in: {date}  {bodyweight} kg");
        }
    }
    Ok(())
}
