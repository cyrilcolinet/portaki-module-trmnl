//! « Écran TRMNL injoignable depuis 6 h » (spec TRMNL §1) : `timelineTasks`. Calculée, jamais
//! stockée, sans case à cocher : la tâche disparaît au premier envoi qui passe.

use chrono::{DateTime, Duration, Utc};
use portaki_sdk::contracts::timeline::{self, TimelineTask, TimelineTasks, TimelineTasksArgs};
use portaki_sdk::host::time;
use portaki_sdk::prelude::*;

use crate::config::ModuleConfig;
use crate::push::{load_state, PushState};

/// Une panne plus courte n'est pas une tâche : l'envoi suivant peut la lever.
const UNREACHABLE_AFTER_HOURS: i64 = 6;

#[portaki_sdk::query(name = "timelineTasks")]
pub fn timeline_tasks(ctx: Context, args: TimelineTasksArgs) -> Result<TimelineTasks> {
    if !ModuleConfig::load(&ctx)?.is_ready() {
        return Ok(TimelineTasks::default());
    }
    let state = load_state().unwrap_or_default();
    Ok(TimelineTasks {
        tasks: unreachable(&args, &state, time::now()?)
            .into_iter()
            .collect(),
    })
}

/// Posée 6 h après le premier échec, ramenée au début de la fenêtre quand c'est plus tôt : une
/// panne oubliée reste à voir, elle ne sort pas de la frise.
fn unreachable(
    args: &TimelineTasksArgs,
    state: &PushState,
    now: DateTime<Utc>,
) -> Option<TimelineTask> {
    let since = DateTime::parse_from_rfc3339(state.failing_since.as_deref()?)
        .ok()?
        .with_timezone(&Utc);
    let late_at = since + Duration::hours(UNREACHABLE_AFTER_HOURS);
    if late_at > now || late_at > args.to {
        return None;
    }
    Some(timeline::task(
        "unreachable",
        late_at.max(args.from),
        args.property_id,
        crate::i18n::text("task.unreachable.title"),
        crate::i18n::text("task.unreachable.context"),
    ))
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn at(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn args() -> TimelineTasksArgs {
        TimelineTasksArgs {
            property_id: Uuid::nil(),
            from: at("2026-09-07T00:00:00Z"),
            to: at("2026-09-21T00:00:00Z"),
            stays: vec![],
        }
    }

    #[test]
    fn a_screen_failing_for_six_hours_becomes_a_task_until_a_push_lands() {
        let mut state = PushState::default();
        state.record(at("2026-09-12T10:00:00Z"), "http_502", false);

        assert!(unreachable(&args(), &state, at("2026-09-12T15:00:00Z")).is_none());
        let task = unreachable(&args(), &state, at("2026-09-12T17:00:00Z")).expect("task");
        assert_eq!(task.id, "unreachable");
        assert_eq!(task.at, at("2026-09-12T16:00:00Z"));
        assert_eq!(task.title.fr, "Écran TRMNL injoignable depuis 6 h");

        state.record(at("2026-09-12T18:00:00Z"), "ok", true);
        assert!(unreachable(&args(), &state, at("2026-09-12T19:00:00Z")).is_none());
    }

    #[test]
    fn an_old_failure_sits_at_the_start_of_the_window() {
        let mut state = PushState::default();
        state.record(at("2026-09-01T10:00:00Z"), "http_502", false);
        let task = unreachable(&args(), &state, at("2026-09-12T10:00:00Z")).expect("task");
        assert_eq!(task.at, args().from);
    }
}
