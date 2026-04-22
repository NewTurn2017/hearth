use anyhow::Result;
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::schedules::{self, NewSchedule, UpdateSchedule};

#[derive(Subcommand)]
pub enum ScheduleCmd {
    /// List schedules. Filter by --month YYYY-MM or --from/--to date range.
    List {
        /// Filter by month (YYYY-MM).
        #[arg(long)]
        month: Option<String>,
        /// Range start date (YYYY-MM-DD). Use with --to.
        #[arg(long, requires = "to")]
        from: Option<String>,
        /// Range end date (YYYY-MM-DD). Use with --from.
        #[arg(long, requires = "from")]
        to: Option<String>,
    },
    /// Get a schedule by id.
    Get { id: i64 },
    /// Create a new schedule entry.
    Create {
        /// Date (YYYY-MM-DD).
        date: String,
        #[arg(long)]
        time: Option<String>,
        #[arg(long)]
        location: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Remind 5 minutes before.
        #[arg(long)]
        remind_5min: bool,
        /// Remind at start.
        #[arg(long)]
        remind_start: bool,
    },
    /// Update a schedule entry.
    Update {
        id: i64,
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        time: Option<String>,
        #[arg(long)]
        location: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Set remind-5min flag.
        #[arg(long)]
        remind_5min: Option<bool>,
        /// Set remind-at-start flag.
        #[arg(long)]
        remind_start: Option<bool>,
    },
    /// Delete a schedule by id.
    Delete { id: i64 },
}

pub fn dispatch(db_path_flag: Option<&str>, sub: ScheduleCmd, pretty: bool) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        ScheduleCmd::List { month, from, to } => {
            let all = if let (Some(f), Some(t)) = (from, to) {
                schedules::list_range(&conn, &f, &t)?
            } else {
                schedules::list(&conn, month.as_deref())?
            };
            let val = serde_json::to_value(&all).unwrap();
            if pretty {
                crate::util::emit_ok_pretty(
                    &val,
                    &["id", "date", "time", "description", "location"],
                );
            } else {
                crate::util::emit_ok(val);
            }
        }
        ScheduleCmd::Get { id } => match schedules::get(&conn, id)? {
            Some(s) => crate::util::emit_ok(serde_json::to_value(&s).unwrap()),
            None => {
                crate::util::emit_err(
                    &format!("schedule {id} not found"),
                    Some("try 'hearth schedule list'"),
                );
                std::process::exit(1);
            }
        },
        ScheduleCmd::Create {
            date,
            time,
            location,
            description,
            notes,
            remind_5min,
            remind_start,
        } => {
            let s = schedules::create(
                &mut conn,
                Source::Cli,
                &NewSchedule {
                    date: &date,
                    time: time.as_deref(),
                    location: location.as_deref(),
                    description: description.as_deref(),
                    notes: notes.as_deref(),
                    remind_before_5min: remind_5min,
                    remind_at_start: remind_start,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&s).unwrap());
        }
        ScheduleCmd::Update {
            id,
            date,
            time,
            location,
            description,
            notes,
            remind_5min,
            remind_start,
        } => {
            let s = schedules::update(
                &mut conn,
                Source::Cli,
                id,
                &UpdateSchedule {
                    date: date.as_deref(),
                    time: time.as_deref(),
                    location: location.as_deref(),
                    description: description.as_deref(),
                    notes: notes.as_deref(),
                    remind_before_5min: remind_5min,
                    remind_at_start: remind_start,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&s).unwrap());
        }
        ScheduleCmd::Delete { id } => {
            schedules::delete(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
