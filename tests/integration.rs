//! Integration-style unit tests with `portaki-test-utils`.

use portaki_sdk::capability;
use portaki_sdk::prelude::EmptyArgs;
use portaki_test_utils::MockContext;
use serial_test::serial;
use trmnl::{
    get_status, on_config_updated, push_now, render_host_main, ConfigUpdatedArgs, DisplayMode,
    ModuleConfig,
};

const WEBHOOK: &str = "https://usetrmnl.com/api/custom_plugins/7f1c-42";

fn configured() -> ModuleConfig {
    ModuleConfig {
        webhook_url: WEBHOOK.to_string(),
        display_mode: DisplayMode::GuestDisplay,
        property_name_override: "Chez Marie".to_string(),
    }
}

/// What the platform sends after the host saves.
fn saved(ctx: &portaki_sdk::context::Context) {
    on_config_updated(ctx.clone(), ConfigUpdatedArgs::default()).expect("onConfigUpdated");
}

/// Before the platform holds the config, the old KV blob is read: an install keeps pushing,
/// and the mode it saved stays.
#[test]
#[serial]
fn an_install_saved_in_kv_keeps_its_settings_until_the_platform_holds_them() {
    let legacy = br#"{"webhook_url":"https://usetrmnl.com/api/custom_plugins/7f1c-42","display_mode":"host_dashboard","property_name_override":""}"#;
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .with_kv("config", legacy.to_vec())
        .run(|ctx| {
            let status = get_status(ctx).expect("getStatus");
            assert!(status.configured);
            assert_eq!(status.display_mode, "host_dashboard");
        });

    // Held by the platform, an empty config is empty: the KV is no longer read.
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .with_kv("config", legacy.to_vec())
        .with_config(&serde_json::json!({}))
        .run(|ctx| {
            let status = get_status(ctx).expect("getStatus");
            assert!(!status.configured);
            assert_eq!(status.display_mode, "guest_display");
        });
}

#[test]
#[serial]
fn a_push_without_a_webhook_says_so_instead_of_failing_the_caller() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .run(|ctx| {
            let outcome = push_now(ctx.clone(), EmptyArgs::default()).expect("pushNow");
            assert!(!outcome.ok);
            assert_eq!(outcome.status, "not_configured");

            let status = get_status(ctx).expect("getStatus");
            assert!(!status.configured);
            assert_eq!(status.push.last_status.as_deref(), Some("not_configured"));
        });
}

#[test]
#[serial]
fn a_configured_push_reaches_the_connector_and_is_written_down() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .with_connector_response("trmnl", "push", r#"{"status":200}"#)
        .with_config(&configured())
        .run(|ctx| {
            saved(&ctx);

            let outcome = push_now(ctx.clone(), EmptyArgs::default()).expect("pushNow");
            assert!(outcome.ok, "{}", outcome.status);

            let status = get_status(ctx).expect("getStatus");
            assert!(status.configured);
            assert_eq!(status.display_mode, "guest_display");
            assert_eq!(status.push.last_status.as_deref(), Some("ok"));
            // Saving pushes too, so the ledger already holds both.
            assert_eq!(status.push.recent.len(), 2);
        });
}

#[test]
#[serial]
fn the_hourly_budget_stops_the_twelfth_push() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .with_connector_response("trmnl", "push", "{}")
        .with_config(&configured())
        .run(|ctx| {
            saved(&ctx); // push 1
            for _ in 0..10 {
                assert!(
                    push_now(ctx.clone(), EmptyArgs::default())
                        .expect("pushNow")
                        .ok
                );
            }

            let refused = push_now(ctx.clone(), EmptyArgs::default()).expect("pushNow");
            assert!(!refused.ok);
            assert_eq!(refused.status, "rate_limited");
            assert_eq!(get_status(ctx).expect("getStatus").push.recent.len(), 11);
        });
}

#[test]
#[serial]
fn the_settings_sheet_shows_the_saved_values_and_both_modes() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .with_connector_response("trmnl", "push", "{}")
        .with_config(&configured())
        .run(|ctx| {
            let surface = render_host_main(ctx).expect("render");
            let json = serde_json::to_string(&surface).expect("serialize");
            assert!(json.contains("\"main\""));
            assert!(json.contains("Chez Marie"));
            // A secret is never sent back to the form.
            assert!(!json.contains(WEBHOOK));
            assert!(json.contains("host_dashboard") && json.contains("guest_display"));
            assert!(json.contains("i18n:host.pushNow"));
        });
}

#[test]
#[serial]
fn the_status_card_admits_when_nothing_was_ever_sent() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .run(|ctx| {
            // L'état vit dans le tiroir, sous le bouton d'envoi.
            let json =
                serde_json::to_string(&render_host_main(ctx).expect("render")).expect("serialize");
            assert!(json.contains("i18n:stats.never"));
            assert!(json.contains("i18n:stats.notConfigured"));
        });
}

/// Un tiers qui refuse : c'est là que le registre du module se lit.
#[test]
#[serial]
fn a_refused_push_is_recorded_without_spending_the_budget() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .with_connector_error("trmnl", "push", "connector_credential_missing")
        .with_config(&configured())
        .run_with(|ctx, host| {
            saved(&ctx);

            let outcome = push_now(ctx.clone(), EmptyArgs::default()).expect("pushNow");
            assert!(!outcome.ok);
            assert!(outcome.status.contains("connector_credential_missing"));

            let status = get_status(ctx).expect("getStatus");
            assert!(
                status.push.recent.is_empty(),
                "a refused push spent the hourly budget"
            );
            assert!(status
                .push
                .last_status
                .as_deref()
                .unwrap_or_default()
                .contains("connector_credential_missing"));

            // Ce qui est parti, et pas seulement qu'il soit parti.
            let calls = host.connector_calls();
            assert_eq!(calls.len(), 2, "one per push attempt, save included");
            assert!(calls[0].args_json.contains("\"plugin_id\":\"7f1c-42\""));
            assert!(calls[0].args_json.contains("Chez Marie"));
        });
}
