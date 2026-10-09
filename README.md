# trmnl

Shows the current stay on a [TRMNL](https://usetrmnl.com) e-ink screen — either as a small
dashboard for the host, or as a display left in the property for the guest.

A Portaki module compiled to wasm, running on the [Portaki module SDK](https://github.com/PortakiApp/portaki-sdk)
3.2.0. It is a rewrite of the original TypeScript module, which lived in `PortakiApp/portaki-modules-old`.

## Module id

`trmnl` — community module, host audience, no guest booklet surface.

## How it works

TRMNL screens are fed by a *Private Plugin*: the plugin owns the layout, and whoever holds its
webhook URL posts the values. This module posts the values.

1. The host creates a Private Plugin in TRMNL with the **Webhook** strategy.
2. They paste its URL into the module settings in Portaki.
3. The module posts a flat map of merge variables whenever the screen would otherwise be stale.

The layout is never in this repository's gift: it lives in the TRMNL plugin, where its author can
change it without republishing a module. `templates/` holds a starting point for both modes.

## Host surfaces

| Surface | Type | What it is |
|---|---|---|
| `main` | `property-module-sheet` | Webhook URL, display mode, name on screen, and a "send now" button |
| `status` | `property-stats-card` | Mode, last send, and what it returned |

## When it sends

| Trigger | Why |
|---|---|
| `updateConfig` | Saving settings repaints the screen, so Save is visibly what it claims to be |
| `pushNow` | The button on the settings sheet |
| `core.booking.confirmed` | A confirmed booking is the one moment a screen is certainly stale |
| the scheduled sync | A countdown drawn yesterday is wrong this morning, with nothing happening to say so |

The schedule is `hostScheduledSync: { hostAction: "pushNow" }` — the platform runs the module's
own command, on its cadence and on demand. It reaches a wasm module only where the platform
dispatches a host action as a command ([portaki-platform#294](https://github.com/PortakiApp/portaki-platform/issues/294));
before that, the declaration is inert rather than wrong.

## Merge variables

Always sent:

| Key | Example |
|---|---|
| `property_name` | `Chez Marie` — the override, else the Portaki property name |
| `display_mode` | `host_dashboard` \| `guest_display` |
| `updated_at` | `2026-09-12T08:00:00+00:00` |
| `updated_time` | `08:00` (UTC) |
| `stay_active` | `true` when the module was handed a stay that has started and not ended |

Sent when the invocation carries a stay:

| Key | Example |
|---|---|
| `checkin_at` / `checkin_date` / `checkin_time` | `2026-09-11T14:00:00+00:00` / `2026-09-11` / `14:00` |
| `checkout_at` / `checkout_date` / `checkout_time` | idem |
| `booking_channel` | `airbnb` |
| `next_stay_countdown_days` | `3` — host dashboard only, never negative |

Times go out in UTC: a wasm module has no timezone database, and a date silently shifted by an
hour is worse on a screen nobody rereads than an explicit UTC stamp. Format them in the template.

## Capabilities

| Kind | Id | Why |
|---|---|---|
| Required | `core.storage` | Settings and the push ledger, in the module's own KV namespace |

Egress is declared as a connector, `trmnl` → `https://usetrmnl.com`, with one operation
`POST /api/custom_plugins/{plugin_id}` and `auth = "none"` — the plugin id in the path is the
whole secret. The manifest lists `connectors:trmnl`; the runtime refuses any connector it does
not name.

## KV

| Key | Shape |
|---|---|
| `config` | Legacy: `{ "webhook_url": "", "display_mode": "host_dashboard", "property_name_override": "" }` — read only while the platform sends no `moduleConfig`, imported once (`legacyConfig`), then deleted (`legacyConfigAdopted`) |
| `push_state` | `{ "recent": [epoch seconds], "last_push_at": "…", "last_status": "ok", "failing_since": "…" }` |

The settings are held by the platform (`#[portaki_sdk::config]`): `webhook_url` is a secret, in
the vault; `display_mode` defaults to `guest_display`.

## Queries / commands

| Operation | Kind | What it does |
|---|---|---|
| `onConfigUpdated` | command | Pushes after the host saves |
| `pushNow` | command | Pushes now; returns `{ ok, status }` |
| `getStatus` | query | Configured or not, plus the push ledger |
| `publishReadiness` | query | A TRMNL Private Plugin URL, a name of 30 characters at most |
| `timelineTasks` | query | « Écran TRMNL injoignable depuis 6 h » in À venir, until a push lands |

## Rate limit

Eleven pushes per hour, counted in KV. TRMNL's free plan allows about twelve, and a screen that
refreshes every fifteen minutes at best gains nothing from a thirteenth — while a module that
trips the plan limit breaks the twelve that mattered. A refused or failed push does not spend the
budget.

## Limitations

- **No guest name, Wi-Fi credentials, or door code.** The old TypeScript module received them in
  an event map enriched by the host. A wasm module reads the property it runs for and the stay it
  was handed — there is no host operation that reads bookings or other modules' settings.
- **The connector needs a credential to exist.** The runtime requires a non-blank token for every
  connector call, even with `auth = "none"` (`connector_credential_missing`). Until that check
  skips `none` ([portaki-platform#293](https://github.com/PortakiApp/portaki-platform/issues/293)),
  a `trmnl` credential provider has to be registered orchestrator-side and bound for the workspace.
- **The rate limit is per property.** Eleven pushes an hour is counted in this module's KV, which
  is scoped to the property — two properties feeding one TRMNL device would each count their own.

## Development

```
cargo test
portaki build --release && portaki lint
```

## License

MIT — see [LICENSE](LICENSE).
