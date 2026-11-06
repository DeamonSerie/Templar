# Templar

Templar is a Substrate-based algorithmic chain for managing debt ledgers. It lets users create ledgers and perform economic actions — deposits, withdrawals, loans, and repayments — where each action type is gated by an immutable schedule chosen by the ledger creator. Schedules are evaluated against network time (time-n ticks), so operations succeed only when the current tick satisfies the configured start, interval, occurrence limit, and optional linked query condition.

## Time and Clocking

Templar reuses `pallet-set-time` (time-n). Network time advances in ticks via `advance_time`, and all economic timing uses `net_time()` ticks.

Trader clocks can be classified as normal, underclocked, or overclocked:

- `set_clock_targets(underclock_hz, overclock_hz)` defines the acceptable measured clock range.
- `record_trader_clock(trader, measured_hz)` stores the measured frequency and classifies it.
- A measured frequency below the underclock target is `Underclocked`; above the overclock target is `Overclocked`; otherwise `Normal`.
- `CheatDetected` is emitted for non-normal traders.

This mirrors other algorithmic chains: by adjusting clock targets and advancing network time, operators can simulate slower (underclocked) or faster (overclocked) environments while keeping consensus time deterministic.

## Build and Test

```bash
cargo test --workspace
cargo build -p templar-node
```
EOF
echo done