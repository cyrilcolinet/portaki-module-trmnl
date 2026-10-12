//! Host dashboard surface — the settings drawer (spec TRMNL §2, planche validée).
//!
//! Deux cartes, Connexion puis Affichage. Le chrome du tiroir (titre, interrupteur, Publier)
//! appartient au dashboard ; « Publier » déclenche `onConfigUpdated`, qui pousse l'écran.

use chrono::DateTime;
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{
    Card, ChoiceList, Field, FieldHint, Form, KeyValue, Page, SecretInput, Stack, TextInput,
};
use portaki_sdk::sdui::surface::Surface;

use crate::config::{DisplayMode, ModuleConfig};
use crate::push::{load_state, PushState};

// « Écran TRMNL injoignable depuis 6 h » dans À venir : `crate::tasks`.
#[portaki_sdk::nav(
    placement = HostPlacement::WorkspaceTimelineTask,
    path = "tasks",
    label_key = "catalog.host.tasks",
    icon = IconName::DangerTriangle
)]
#[portaki_sdk::surface(host, id = "main", placement = HostPlacement::PropertyModuleSheet)]
pub fn render_host_main(ctx: HostContext) -> Result<Surface> {
    let config = ModuleConfig::load(&ctx)?;
    let problems = config.problems();
    // Le champ, avec le message de `problems` sous lui s'il y en a un. Une URL encore vide
    // n'est pas une faute à souligner : c'est le tiroir qu'on ouvre pour la première fois.
    let named = |name: &str| {
        let field = Field::new().name(name);
        let blank = name == "webhook_url" && config.webhook_url.trim().is_empty();
        match problems.iter().find(|(field, _)| *field == name) {
            Some((_, key)) if !blank => {
                field.error(crate::i18n::text(key).get(&ctx.locale).to_string())
            }
            _ => field,
        }
    };

    let mut connection: Vec<Component> = vec![named("webhook_url")
        .label("i18n:host.webhookUrl.label")
        .child(
            SecretInput::new()
                .name("webhook_url")
                // Never sent back: blank keeps the saved URL.
                .value(String::new())
                .placeholder("i18n:host.webhookUrl.placeholder"),
        )
        .child(FieldHint::new().text("i18n:host.webhookUrl.hint"))
        .into()];
    // Un `KeyValue` et non un champ : un champ nommé partirait avec le formulaire.
    if let Some(at) = last_sent(&load_state().unwrap_or_default()) {
        connection.push(
            KeyValue::new()
                .key("i18n:stats.lastPush")
                .value(at)
                .mono(true)
                .into(),
        );
    }

    let display: Vec<Component> = vec![
        Field::new()
            .name("display_mode")
            .label("i18n:host.displayMode.label")
            .child(
                ChoiceList::new()
                    .name("display_mode")
                    .layout(ChoiceListLayout::Segmented)
                    .choices(vec![
                        ChoiceOption::new(
                            DisplayMode::HostDashboard.as_str(),
                            "i18n:host.displayMode.hostDashboard",
                        ),
                        ChoiceOption::new(
                            DisplayMode::GuestDisplay.as_str(),
                            "i18n:host.displayMode.guestDisplay",
                        ),
                    ])
                    .value(config.display_mode.as_str()),
            )
            .child(FieldHint::new().text("i18n:host.displayMode.hint"))
            .into(),
        named("property_name_override")
            .label("i18n:host.propertyName.label")
            .required(false)
            .child(
                TextInput::new()
                    .name("property_name_override")
                    .value(config.property_name_override.clone())
                    .placeholder("i18n:host.propertyName.placeholder"),
            )
            .child(FieldHint::new().text("i18n:host.propertyName.hint"))
            .into(),
    ];

    let cards: Vec<Component> = vec![
        Card::new()
            .title("i18n:host.section.connection")
            .children(connection)
            .into(),
        Card::new()
            .title("i18n:host.section.display")
            .children(display)
            .into(),
    ];

    // No Page title / Save — the modules sheet owns chrome + footer Save.
    Ok(
        Surface::new(Page::new().child(Form::new().child(Stack::new().gap(16.0).children(cards))))
            .with_id(MAIN),
    )
}

/// L'heure du dernier envoi qui a abouti, en UTC (pas de base de fuseaux en wasm). Rien tant
/// qu'aucun n'a abouti : une tentative refusée n'est pas un envoi.
fn last_sent(state: &PushState) -> Option<String> {
    let raw = state.last_ok_at.clone().or_else(|| {
        // Un registre d'avant `last_ok_at` : la dernière tentative compte si elle a abouti.
        (state.last_status.as_deref() == Some("ok"))
            .then(|| state.last_push_at.clone())
            .flatten()
    })?;
    let at = DateTime::parse_from_rfc3339(raw.trim()).ok()?;
    Some(at.format("%Y-%m-%d %H:%M UTC").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_push_that_landed_counts_as_the_last_send() {
        let mut state = PushState::default();
        assert_eq!(last_sent(&state), None);

        state.last_push_at = Some("2026-10-12T14:00:00+00:00".into());
        state.last_status = Some("http_502".into());
        assert_eq!(last_sent(&state), None, "a failed attempt is not a send");

        // A ledger written before `last_ok_at` existed.
        state.last_status = Some("ok".into());
        assert_eq!(last_sent(&state).as_deref(), Some("2026-10-12 14:00 UTC"));

        state.last_ok_at = Some("2026-10-12T09:30:00+00:00".into());
        state.last_status = Some("rate_limited".into());
        assert_eq!(last_sent(&state).as_deref(), Some("2026-10-12 09:30 UTC"));
    }
}
