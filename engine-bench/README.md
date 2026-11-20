# Engine hour-boundary bench

The Rust engine's tick-cost bench. It lays a ~20k-unit tower (a dense office
grid, a sparse fast food row, parking basements and standard shafts), warms
up for 30 ticks, then walks one in-game day a minute at a time and reports the
average regular tick and the 24 ticks that cross an hour boundary. Then it
times each panel getter (the HUD stats, the stats report, housekeeping
coverage, a unit's detail, the congestion hotspot, the onboarding facts and
the four floor heatmaps) on the walked tower, 30 runs each, and reports the
medians.

```sh
cd engine-bench
cargo run --release
```

It is the same tower and walk as the TypeScript engine's bench it replaced,
so the series the audit started in July 2026 carries on. `.github/workflows/engine-bench.yml` runs it every Monday
and puts the numbers in the job summary.

Timings are report-only. Shared runners are noisy, so read the series as a
trend. The run fails only when the tower does not build as planned, the walk
does not cross all 24 hour boundaries, or a panel getter's median passes
50 ms, a loose tripwire for a getter gone pathological (healthy medians are
well under a millisecond).

The bench is its own crate, outside the engine crate's `include` list, on
purpose: what the engine crate packages is part of the engine source hash, and
a bench is not a simulation input.
