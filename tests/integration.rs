//! Integration-style unit tests with `portaki-test-utils`.

use portaki_sdk::capability;
use portaki_sdk::prelude::EmptyArgs;
use portaki_test_utils::MockContext;
use serial_test::serial;
use trmnl::{get_config, get_status, push_now, render_host_main, update_config, UpdateConfigArgs};

const WEBHOOK: &str = "https://usetrmnl.com/api/custom_plugins/7f1c-42";

fn configured() -> UpdateConfigArgs {
    UpdateConfigArgs {
        webhook_url: WEBHOOK.to_string(),
        display_mode: "guest_display".to_string(),
        property_name_override: "Chez Marie".to_string(),
    }
}

#[test]
#[serial]
fn saving_the_settings_makes_them_readable_again() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .with_connector_response("trmnl", "push", "{}")
        .run(|ctx| {
            update_config(ctx.clone(), configured()).expect("updateConfig");

            let config = get_config(ctx).expect("getConfig");
            assert_eq!(config.webhook_url, WEBHOOK);
            assert_eq!(config.display_mode.as_str(), "guest_display");
            assert_eq!(config.property_name_override, "Chez Marie");
        });
}

#[test]
#[serial]
fn a_webhook_from_somewhere_else_is_refused_before_it_is_stored() {
    MockContext::host()
        .with_capabilities(&[capability::core::STORAGE])
        .run(|ctx| {
            let args = UpdateConfigArgs {
                webhook_url: "https://example.com/hook".to_string(),
                ..UpdateConfigArgs::default()
            };
            update_config(ctx.clone(), args).expect_err("refused");

            // Nothing was written: the module is still unconfigured.
            assert!(!get_status(ctx).expect("getStatus").configured);
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
        .run(|ctx| {
            update_config(ctx.clone(), configured()).expect("updateConfig");

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
        .run(|ctx| {
            update_config(ctx.clone(), configured()).expect("updateConfig"); // push 1
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
        .run(|ctx| {
            update_config(ctx.clone(), configured()).expect("updateConfig");

            let surface = render_host_main(ctx);
            let json = serde_json::to_string(&surface).expect("serialize");
            assert!(json.contains("\"main\""));
            assert!(json.contains(WEBHOOK));
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
            let json = serde_json::to_string(&render_host_main(ctx)).expect("serialize");
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
        .run_with(|ctx, host| {
            update_config(ctx.clone(), configured()).expect("updateConfig");

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
