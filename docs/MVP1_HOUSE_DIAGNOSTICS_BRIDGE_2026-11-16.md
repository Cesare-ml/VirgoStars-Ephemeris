# Optional serialized house diagnostics — 2026-11-16

**Status: implemented and tested locally with the unchanged legacy backend;
not installed in Virgo Stars, not production activated.** This bridge prepares
a reviewable response contract for the missing polar warnings identified in
[the chart diagnostic](MVP1_BACKEND_CHART_DIAGNOSTIC_2026-11-16.md).

## Contract

`createHouseDiagnosticSession(facade)` returns a wrapped `facade` and a
`run(calculate)` method. Inject the wrapped facade into the synchronous legacy
calculator and execute one chart inside `run`:

```js
const { createHouseDiagnosticSession } = require('virgostars-ephemeris-node');
const session = createHouseDiagnosticSession(facade);
// Inject session.facade into a backend calculator using its own explicit wiring.
const response = session.run(() => calculateChart());
// JSON.stringify(response) retains response.diagnostics independently of the
// legacy chart's non-enumerable house metadata.
```

The legacy result itself and its enumerated keys are unchanged. The separate
JSON-safe envelope has schema `virgostars-house-diagnostics/1` and an `events`
array. Each event contains only `operation`, `code`, `system` and `count`:

- `FALLBACK_PORPHYRY` is recorded per requested house system when the facade
  actually reports fallback. `POLAR_CIRCLE_LATITUDE` is reported once per scope
  with system `ALL`, even if several house systems are requested.
- `E_POSITION_UNAVAILABLE` and other recognized house errors are counted per
  operation and system. Unrecognized external error codes become `E_OTHER`.
  The legacy `swe_houses_pos` signature does not identify the celestial body;
  these diagnostics deliberately do not guess it.
- No epoch, coordinate, angle, source path or free-form error text is copied
  into diagnostics. The `result` remains the complete caller-owned chart and
  may itself contain personal birth data; handle it under the application's
  existing privacy rules.

Collection is synchronous and scoped to one `run` call. Nested scopes and
async functions/returned promises are rejected. State is cleared on success or
exception; calls made outside a scope are forwarded but not recorded. Use one
session per synchronous calculation context, and do not share one across
concurrent or asynchronous requests. This does not capture unrelated planetary,
catalog or backend errors.

## Evidence and remaining integration

Unit tests exercise aggregation, JSON round-trips, privacy of diagnostics,
unknown errors, scope isolation, exceptions and misuse. An opt-in local test
loads the actual unchanged backend with a test-only module injection and shows
an ordinary chart with no events and a polar chart with Placidus/Koch fallback
plus unavailable-position events; JSON retains the diagnostics.

The legacy backend currently hard-codes its `swisseph` import. The test uses a
process-local module hook solely to prove the behavior; that hook is **not** a
production integration pattern. Application owners still need an explicit
dependency-injection adapter, a versioned HTTP/consumer contract, per-body
handling where required, product warning text, error policy, parallel validation
and rollout/rollback. No existing HTTP response or production engine was
changed by this bridge.
