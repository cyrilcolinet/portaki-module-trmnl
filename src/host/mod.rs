//! Host dashboard surfaces — the settings sheet and the status card.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{
    Button, Card, Field, FieldHint, Form, InfoBanner, KeyValue, Page, SecretInput, Select, Stack,
    TextInput,
};
use portaki_sdk::sdui::surface::Surface;

use crate::config::{DisplayMode, ModuleConfig};
use crate::ids;
use crate::push::load_state;

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
    let push_action = ids::module_id().command_empty(crate::commands::PUSH_NOW);

    let form_children: Vec<Component> = vec![
        InfoBanner::new()
            .title("i18n:host.banner.title")
            .message("i18n:host.banner.message")
            .into(),
        Card::new()
            .title("i18n:host.section.plugin")
            .subtitle("i18n:host.section.plugin.help")
            .icon(IconName::Link)
            .children(vec![
                named("webhook_url")
                    .label("i18n:host.webhookUrl.label")
                    .child(
                        SecretInput::new()
                            .name("webhook_url")
                            // Never sent back: blank keeps the saved URL.
                            .value(String::new())
                            .placeholder("i18n:host.webhookUrl.placeholder"),
                    )
                    .into(),
                FieldHint::new().text("i18n:host.webhookUrl.hint").into(),
            ])
            .into(),
        Card::new()
            .title("i18n:host.section.screen")
            .subtitle("i18n:host.section.screen.help")
            .icon(IconName::Grid)
            .children(vec![
                Field::new()
                    .name("display_mode")
                    .label("i18n:host.displayMode.label")
                    .child(
                        Select::new()
                            .name("display_mode")
                            .options(vec![
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
                    .into(),
                named("property_name_override")
                    .label("i18n:host.propertyName.label")
                    .child(
                        TextInput::new()
                            .name("property_name_override")
                            .value(config.property_name_override.clone())
                            .placeholder("i18n:host.propertyName.placeholder"),
                    )
                    .into(),
            ])
            .into(),
    ];

    let mut children = form_children;
    // The button sits outside no form of its own: the sheet merges the fields above at click
    // time, so "send now" pushes what is on screen rather than what was last saved.
    children.push(
        Stack::new()
            .direction(StackDirection::Horizontal)
            .gap(10.0)
            .children(vec![Button::new()
                .label("i18n:host.pushNow")
                .variant(ButtonVariant::Outline)
                .action(push_action)
                .into()])
            .into(),
    );
    children.push(FieldHint::new().text("i18n:host.main.help").into());
    children.push(status_card(&config));

    // No Page title / Save — the modules sheet owns chrome + footer Save.
    Ok(Surface::new(
        Page::new().child(Form::new().child(Stack::new().gap(16.0).children(children))),
    )
    .with_id(MAIN))
}

/// L'état de l'écran, dans le tiroir : s'il est alimenté, et ce qu'a donné le dernier envoi.
///
/// Dans le tiroir et non en tuile de statistiques : la spec n'en prévoit pas pour TRMNL, et
/// c'est là que l'hôte vient vérifier après « Envoyer maintenant ». Des `KeyValue` et non des
/// champs : un champ nommé partirait avec le formulaire à l'enregistrement.
pub fn status_card(config: &ModuleConfig) -> Component {
    let state = load_state().unwrap_or_default();

    let mode = if config.is_ready() {
        match config.display_mode {
            DisplayMode::HostDashboard => "i18n:host.displayMode.hostDashboard",
            DisplayMode::GuestDisplay => "i18n:host.displayMode.guestDisplay",
        }
        .to_string()
    } else {
        "i18n:stats.notConfigured".to_string()
    };
    let last_push = state
        .last_push_at
        .filter(|at| !at.trim().is_empty())
        .unwrap_or_else(|| "i18n:stats.never".to_string());
    let status = match state.last_status.as_deref() {
        Some("ok") => "i18n:stats.status.ok".to_string(),
        Some("rate_limited") => "i18n:stats.status.rateLimited".to_string(),
        Some("not_configured") | None => "i18n:stats.status.idle".to_string(),
        Some(other) => other.to_string(),
    };

    Card::new()
        .title("i18n:stats.title")
        .subtitle("i18n:stats.subtitle")
        .icon(IconName::Gauge)
        .children(vec![
            KeyValue::new().key("i18n:stats.mode").value(mode).into(),
            KeyValue::new()
                .key("i18n:stats.lastPush")
                .value(last_push)
                .into(),
            KeyValue::new()
                .key("i18n:stats.lastStatus")
                .value(status)
                .into(),
        ])
        .into()
}
