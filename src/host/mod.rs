//! Host dashboard surfaces — the settings sheet and the status card.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{
    Button, Card, Field, FieldHint, Form, InfoBanner, Page, SecretInput, Select, Stack, Text,
    TextInput,
};
use portaki_sdk::sdui::surface::Surface;

use crate::config::{load_config, DisplayMode};
use crate::ids;
use crate::push::load_state;

#[portaki_sdk::surface(host, id = "main")]
pub fn render_host_main(_ctx: HostContext) -> Surface {
    let config = load_config().unwrap_or_default();
    let push_action = ids::module_id().command_empty(ids::PUSH_NOW);

    let form_children: Vec<Component> = vec![
        InfoBanner::new()
            .title("i18n:host.banner.title")
            .message("i18n:host.banner.message")
            .into(),
        Card::new()
            .title("i18n:host.section.plugin")
            .subtitle("i18n:host.section.plugin.help")
            .icon("Link")
            .children(vec![
                Field::new()
                    .name("webhook_url")
                    .label("i18n:host.webhookUrl.label")
                    .child(
                        SecretInput::new()
                            .name("webhook_url")
                            .value(config.webhook_url.clone())
                            .placeholder("i18n:host.webhookUrl.placeholder"),
                    )
                    .into(),
                FieldHint::new().text("i18n:host.webhookUrl.hint").into(),
            ])
            .into(),
        Card::new()
            .title("i18n:host.section.screen")
            .subtitle("i18n:host.section.screen.help")
            .icon("Grid")
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
                Field::new()
                    .name("property_name_override")
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

    // No Page title / Save — the modules sheet owns chrome + footer Save.
    Surface::new(Page::new().child(Form::new().child(Stack::new().gap(16.0).children(children))))
        .with_id(ids::HOST_MAIN)
}

/// The stats strip: whether the screen is fed, and what the last attempt did.
#[portaki_sdk::surface(host, id = "status")]
pub fn render_host_status(_ctx: HostContext) -> Surface {
    let config = load_config().unwrap_or_default();
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

    Surface::new(
        Page::new().child(
            Card::new()
                .title("i18n:stats.title")
                .subtitle("i18n:stats.subtitle")
                .icon("Monitor")
                .children(vec![
                    Field::new()
                        .name("display_mode")
                        .label("i18n:stats.mode")
                        .child(Text::new().text(mode).variant(TextVariant::Body))
                        .into(),
                    Field::new()
                        .name("last_push_at")
                        .label("i18n:stats.lastPush")
                        .child(Text::new().text(last_push).variant(TextVariant::Caption))
                        .into(),
                    Field::new()
                        .name("last_status")
                        .label("i18n:stats.lastStatus")
                        .child(Text::new().text(status).variant(TextVariant::Caption))
                        .into(),
                ]),
        ),
    )
    .with_id(ids::HOST_STATUS)
}
