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

There is no periodic push. `hostScheduledSync` without `platformFetch` runs the *legacy Java host
backend*, not a wasm command, and its `platformFetch` variant only fetches URLs — neither one can
drive an outbound POST from a wasm module today. See [Limitations](#limitations).

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
| `config` | `{ "webhook_url": "", "display_mode": "host_dashboard", "property_name_override": "" }` |
| `push_state` | `{ "recent": [epoch seconds], "last_push_at": "…", "last_status": "ok" }` |

## Queries / commands

| Operation | Kind | What it does |
|---|---|---|
| `updateConfig` | command | Validates and saves the settings, then pushes |
| `pushNow` | command | Pushes now; returns `{ ok, status }` |
| `getConfig` | query | The settings, as saved |
| `getStatus` | query | Configured or not, plus the push ledger |

## Rate limit

Eleven pushes per hour, counted in KV. TRMNL's free plan allows about twelve, and a screen that
refreshes every fifteen minutes at best gains nothing from a thirteenth — while a module that
trips the plan limit breaks the twelve that mattered. A refused or failed push does not spend the
budget.

## Limitations

Compared with the original TypeScript module, three things cannot be expressed on SDK 3.2.0:

- **No guest name, Wi-Fi credentials, or door code.** The old module received them in an event
  map enriched by the host. A wasm module reads the property it runs for and the stay it was
  handed — there is no host operation that reads bookings or other modules' settings.
- **No periodic refresh.** See [When it sends](#when-it-sends).
- **The connector needs a credential to exist.** The runtime requires a non-blank token for every
  connector call, even with `auth = "none"` (`connector_credential_missing`). Until that check
  skips `none`, a `trmnl` credential provider has to be registered orchestrator-side and bound for
  the workspace.

## Development

```
cargo test
portaki build --release && portaki lint
```

## License

MIT — see [LICENSE](LICENSE).
