//! Admin CLI: works directly with YDB on behalf of a user.

// The ydb SDK futures are deeply nested, see https://github.com/rust-lang/rust/issues/152942
#![recursion_limit = "256"]

use std::str::FromStr;
use std::time::SystemTime;

use anyhow::Context;
use bodyweight_domain::{
    Bodyweight, BodyweightRatio, CustomExercise, ExerciseEntry, ExerciseId, Period, Pr, PrValue,
    Reps, Set, StandardExercise, WeighIn, Weight, Workout, WorkoutId, bodyweight_on, pr,
};
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
    /// Workouts with their exercise entries and sets.
    #[command(subcommand)]
    Workouts(WorkoutsCommand),
    /// Show the current PRs in the standard exercises.
    Prs,
    /// Weigh-ins: the user's bodyweight by date.
    #[command(subcommand)]
    WeighIns(WeighInsCommand),
    /// Exercises: the standard ones and the user's custom exercises.
    #[command(subcommand)]
    Exercises(ExercisesCommand),
}

#[derive(Subcommand)]
enum WorkoutsCommand {
    /// List workouts, newest first.
    List,
    /// Show a workout with its exercise entries and sets.
    Show {
        /// Workout id, as printed by `workouts list`.
        id: WorkoutId,
    },
    /// Create an empty workout and print its id.
    Add {
        /// Date in YYYY-MM-DD format. Defaults to today in Europe/Moscow.
        #[arg(long)]
        date: Option<NaiveDate>,
        /// Free text attached to the workout.
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Add an exercise entry with its sets to the end of a workout.
    AddEntry {
        /// Workout id, as printed by `workouts list`.
        id: WorkoutId,
        /// A standard exercise (pull-ups, bench-press, squat, deadlift) or the name of
        /// a custom exercise, see `exercises list`.
        exercise: String,
        /// Sets in order: REPS@KG with a weight, e.g. 5@72.5, or just REPS without one.
        #[arg(value_name = "SET")]
        sets: Vec<SetArg>,
    },
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

#[derive(Subcommand)]
enum ExercisesCommand {
    /// List the standard exercises and the user's custom exercises.
    List,
}

/// A set as typed on the command line: `5@72.5` or `12`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SetArg(Set);

impl FromStr for SetArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (reps, weight) = match s.split_once('@') {
            Some((reps, weight)) => (reps, Some(weight)),
            None => (s, None),
        };
        let reps = reps
            .parse::<u32>()
            .map_err(|_| "a set is REPS@KG or REPS, e.g. 5@72.5 or 12".to_owned())?;
        Ok(Self(Set {
            reps: Reps::new(reps).map_err(|e| e.to_string())?,
            weight: weight
                .map(Weight::from_str)
                .transpose()
                .map_err(|e| e.to_string())?,
        }))
    }
}

/// `5 × 72.5 kg`, or `12` for a set without a weight.
fn show_set(set: Set) -> String {
    match set.weight {
        Some(weight) => format!("{} × {weight} kg", set.reps.get()),
        None => set.reps.get().to_string(),
    }
}

/// Names of the exercises a user can have in their workouts.
struct Catalog {
    custom: Vec<CustomExercise>,
}

impl Catalog {
    fn name(&self, id: &ExerciseId) -> &str {
        match id {
            ExerciseId::Standard(exercise) => exercise.name(),
            ExerciseId::Custom(id) => self
                .custom
                .iter()
                .find(|e| e.id == *id)
                .map_or("Deleted exercise", |e| e.name.as_str()),
        }
    }

    /// Finds an exercise by what was typed: the id or the name of a standard exercise, or
    /// the name of a custom one. Letter case does not matter.
    fn find(&self, typed: &str) -> anyhow::Result<ExerciseId> {
        let same = |name: &str| name.to_lowercase() == typed.trim().to_lowercase();
        if let Some(exercise) = StandardExercise::ALL
            .into_iter()
            .find(|e| same(e.id()) || same(e.name()))
        {
            return Ok(ExerciseId::Standard(exercise));
        }
        let mut found = self.custom.iter().filter(|e| same(e.name.as_str()));
        match (found.next(), found.next()) {
            (Some(exercise), None) => {
                anyhow::ensure!(!exercise.archived, "exercise {typed:?} is archived");
                Ok(ExerciseId::Custom(exercise.id.clone()))
            }
            (Some(_), Some(_)) => anyhow::bail!("several custom exercises are named {typed:?}"),
            _ => anyhow::bail!("no exercise {typed:?}, see `bodyweight exercises list`"),
        }
    }
}

/// The start of the text on one line, at most `max` characters.
fn excerpt(text: &str, max: usize) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= max {
        return line;
    }
    let start: String = line.chars().take(max - 1).collect();
    format!("{start}…")
}

/// `120 kg — 90×10, 2026-10-12` for the Big Three, `9 reps, 2026-10-18` for pull-ups.
fn show_pr(pr: Option<Pr>) -> String {
    let Some(pr) = pr else {
        return "—".into();
    };
    match (pr.value, pr.set.weight) {
        (PrValue::Estimated1Rm(estimate), Some(weight)) => {
            format!(
                "{estimate} kg — {weight}×{}, {}",
                pr.set.reps.get(),
                pr.date
            )
        }
        _ => format!("{} reps, {}", pr.set.reps.get(), pr.date),
    }
}

fn today() -> NaiveDate {
    chrono::Utc::now().with_timezone(&TIME_ZONE).date_naive()
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
        Command::Workouts(WorkoutsCommand::List) => {
            let workouts = storage.list_workouts(user).await?;
            if workouts.is_empty() {
                println!("No workouts yet.");
                return Ok(());
            }
            let catalog = Catalog {
                custom: storage.list_custom_exercises(user).await?,
            };
            println!("{:<32}  {:<10}  EXERCISES", "ID", "DATE");
            for w in workouts {
                let mut names: Vec<&str> = Vec::new();
                for entry in &w.entries {
                    let name = catalog.name(&entry.exercise);
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                let mut line = format!("{:<32}  {:<10}  {}", w.id, w.date, names.join(", "));
                if !w.note.is_empty() {
                    line.push_str(&format!("  [{}]", excerpt(&w.note, 40)));
                }
                println!("{}", line.trim_end());
            }
        }
        Command::Workouts(WorkoutsCommand::Show { id }) => {
            let workout = storage
                .get_workout(user, &id)
                .await?
                .with_context(|| format!("no workout {id}"))?;
            let catalog = Catalog {
                custom: storage.list_custom_exercises(user).await?,
            };
            let bodyweight = bodyweight_on(&storage.list_weigh_ins(user).await?, workout.date);
            println!("{}", workout.date);
            if !workout.note.is_empty() {
                println!("{}", workout.note);
            }
            for entry in &workout.entries {
                println!("\n{}", catalog.name(&entry.exercise));
                for set in &entry.sets {
                    let ratio = match (set.weight, bodyweight) {
                        (Some(weight), Some(bodyweight)) => {
                            format!("  ({} bw)", BodyweightRatio::of(weight, bodyweight))
                        }
                        _ => String::new(),
                    };
                    println!("  {}{ratio}", show_set(*set));
                }
            }
        }
        Command::Workouts(WorkoutsCommand::Add { date, note }) => {
            let id: WorkoutId = hex::encode(rand::random::<[u8; 16]>()).parse()?;
            let date = date.unwrap_or_else(today);
            let workout = Workout::new(id, date, &note, Vec::new())?;
            storage
                .put_workout(user, &workout, SystemTime::now())
                .await?;
            println!("Created workout {} on {date}", workout.id);
        }
        Command::Workouts(WorkoutsCommand::AddEntry { id, exercise, sets }) => {
            let catalog = Catalog {
                custom: storage.list_custom_exercises(user).await?,
            };
            let exercise = catalog.find(&exercise)?;
            let workout = storage
                .get_workout(user, &id)
                .await?
                .with_context(|| format!("no workout {id}"))?;
            let sets: Vec<Set> = sets.into_iter().map(|s| s.0).collect();
            let summary = sets.iter().map(|s| show_set(*s)).collect::<Vec<_>>();

            let mut entries = workout.entries;
            entries.push(ExerciseEntry {
                exercise: exercise.clone(),
                sets,
            });
            let workout = Workout::new(workout.id, workout.date, &workout.note, entries)?;
            storage
                .put_workout(user, &workout, SystemTime::now())
                .await?;
            println!(
                "Added {} to the workout on {}: {}",
                catalog.name(&exercise),
                workout.date,
                if summary.is_empty() {
                    "no sets".to_owned()
                } else {
                    summary.join(", ")
                }
            );
        }
        Command::Prs => {
            let workouts = storage.list_workouts(user).await?;
            let today = today();
            println!("{:<12}  {:<30}  ALL TIME", "EXERCISE", "90 DAYS");
            for exercise in StandardExercise::ALL {
                let of = |period| show_pr(pr(exercise, period, &workouts, today));
                let line = format!(
                    "{:<12}  {:<30}  {}",
                    exercise.name(),
                    of(Period::Days90),
                    of(Period::AllTime)
                );
                println!("{}", line.trim_end());
            }
        }
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
            let date = date.unwrap_or_else(today);
            storage
                .put_weigh_in(user, WeighIn { date, bodyweight })
                .await?;
            println!("Saved weigh-in: {date}  {bodyweight} kg");
        }
        Command::Exercises(ExercisesCommand::List) => {
            let custom = storage.list_custom_exercises(user).await?;
            println!("{:<8}  NAME", "KIND");
            for e in StandardExercise::ALL {
                println!("{:<8}  {}", "standard", e.name());
            }
            for e in custom {
                let archived = if e.archived { "  (archived)" } else { "" };
                println!("{:<8}  {}{archived}", "custom", e.name);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(arg: &str) -> Result<String, String> {
        arg.parse::<SetArg>().map(|s| show_set(s.0))
    }

    fn catalog() -> Catalog {
        let custom = |id: &str, name: &str, archived| CustomExercise {
            id: id.into(),
            name: name.parse().unwrap(),
            archived,
        };
        Catalog {
            custom: vec![
                custom("a1", "Seated row", false),
                custom("a2", "Dips", true),
                custom("a3", "Curl", false),
                custom("a4", "curl", false),
            ],
        }
    }

    #[test]
    fn a_set_is_reps_with_an_optional_weight() {
        assert_eq!(set("5@72.5").as_deref(), Ok("5 × 72.5 kg"));
        assert_eq!(set("12").as_deref(), Ok("12"));
        assert_eq!(set("1@100").as_deref(), Ok("1 × 100 kg"));
    }

    #[test]
    fn invalid_sets_are_rejected() {
        for arg in ["", "x", "5x70", "5@", "@70", "0", "5@70.1", "5@0", "5@70@1"] {
            assert!(set(arg).is_err(), "{arg:?}");
        }
    }

    #[test]
    fn an_exercise_is_found_by_id_or_name() {
        let catalog = catalog();
        let squat = ExerciseId::Standard(StandardExercise::Squat);
        assert_eq!(catalog.find("squat").unwrap(), squat);
        let bench = ExerciseId::Standard(StandardExercise::BenchPress);
        assert_eq!(catalog.find("bench-press").unwrap(), bench);
        assert_eq!(catalog.find("Bench Press").unwrap(), bench);
        let row = ExerciseId::Custom("a1".into());
        assert_eq!(catalog.find(" seated ROW ").unwrap(), row);
        assert_eq!(catalog.name(&row), "Seated row");
        assert_eq!(catalog.name(&squat), "Squat");
        assert_eq!(
            catalog.name(&ExerciseId::Custom("gone".into())),
            "Deleted exercise"
        );
    }

    #[test]
    fn unknown_archived_and_ambiguous_exercises_are_refused() {
        let catalog = catalog();
        for typed in ["lunges", "a1", "Dips", "Curl"] {
            assert!(catalog.find(typed).is_err(), "{typed:?}");
        }
    }

    #[test]
    fn prs_are_shown_as_in_the_spec() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        let big = "10@90".parse::<SetArg>().unwrap().0;
        let estimate = bodyweight_domain::Estimated1Rm::of(big.weight.unwrap(), big.reps);
        let pr = Pr {
            value: PrValue::Estimated1Rm(estimate),
            set: big,
            date,
        };
        assert_eq!(show_pr(Some(pr)), "120 kg — 90×10, 2026-10-12");

        let reps = "9".parse::<SetArg>().unwrap().0;
        let pr = Pr {
            value: PrValue::MaxReps(reps.reps),
            set: reps,
            date,
        };
        assert_eq!(show_pr(Some(pr)), "9 reps, 2026-10-12");
        assert_eq!(show_pr(None), "—");
    }

    #[test]
    fn a_note_is_shortened_to_one_line() {
        assert_eq!(excerpt("legs\n  felt strong", 40), "legs felt strong");
        assert_eq!(excerpt("abcdefgh", 5), "abcd…");
    }
}
