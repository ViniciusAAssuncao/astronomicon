# Astronomicon — Checkpoint 25: Release flight tick bottleneck diagnosis

Date: 2026-10-15

## Scope

Measurement-only checkpoint. No change to mathematical equations, DOP853, N-body source selection, atmospheric response, trajectory persistence, DTOs, FFI, or Qt rendering.

Analyzed backend branch `dev-version0.7-unstable`, HEAD `2c1ba1a`, with pre-existing uncommitted CP23/CP24 changes. The benchmark was compiled with `cargo build --release -p rocketcon-sim --bin flight_tick_bench -j 2`. Workspace Release profile has `opt-level = 3` and `lto = true`. Measurements were taken on Windows with the existing `flight_tick_bench` executable and disposable scenario databases. The fixture advances physical time by 0.02 seconds per tick. This **differs** from the actual Qt FlightController physical timestep, which is 1/60 second; the results must not be used as a direct 60 Hz physics acceptance test.

These are short, scenario-specific measurements, not a guarantee of 60 FPS in the desktop app, not a sustained-run SOI validation, and not a benchmark of the FFI or Qt render loop.

## Release tick results

| Scenario and profile | Ticks | Median tick, ms | p95 tick, ms | First tick, ms |
| --- | ---: | ---: | ---: | ---: |
| Powered vacuum, `stages` | 40 | 8.02 | 8.54 | 12.16 |
| Powered vacuum, unprofiled | 40 | 8.20 | 9.09 | 12.55 |
| Powered vacuum, `sql` | 18 | 9.02 | 10.64 | 49.40 |
| Coasting, `stages` | 30 | 6.91 | 9.07 | 9.83 |
| Atmosphere, `stages` | 30 | 16.06 | 17.28 | 18.26 |
| Atmosphere, unprofiled | 40 | 17.00 | 24.81 | 18.79 |
| Powered atmosphere, `stages` | 30 | 15.15 | 16.11 | 18.12 |

The `sql` mode uses a different connection and tracing path. Do not subtract its median from the `stages` median to claim an exact overhead. The median excludes the first tick; each per-stage median is computed separately and does not necessarily sum to the tick median.

The 60 FPS visual frame budget is approximately 16.67 ms. The benchmark uses a 20 ms simulation step. The Qt FlightController requests a 1/60 s physics step and has a separate approximately 16 ms presentation timer. These are not the same timestep. The benchmark is not an observed frame pacing measurement.

### Stage medians in Release, ms

| Stage | Powered vacuum | Atmosphere | Powered atmosphere |
| --- | ---: | ---: | ---: |
| Initial state/environment snapshot | 1.58 | 1.97 | 2.01 |
| Power and battery | 1.57 | 1.70 | 1.67 |
| Component heat inputs | 0.46 | 0.54 | 0.49 |
| Initial aero and contact | 0.18 | 3.49 | 3.17 |
| Powered N-body integration | 1.76 | Not selected | Not selected |
| Dynamics or coast | 0.17 | 1.34 | 1.23 |
| Final aero and shield | 0.17 | 3.73 | 3.23 |
| Thermal network | 1.72 | 1.74 | 1.79 |
| Gravity, max-Q, report | 0.01 | 1.01 | 1.00 |

The two atmospheric aero-related stages have individual medians of 3.49 ms and 3.73 ms (3.17 ms and 3.23 ms for powered atmosphere). They are a stronger optimization candidate than the current 1.76 ms N-body-powered stage in vacuum. These stage names include adjacent work such as contact, final-ephemeris handling, and shield processing; finer instrumentation is needed before attributing the entire time to a single aerodynamic function.

The `powered` atmospheric fixture enters the specialized-force regime, not the N-body-powered regime. These results cannot bound a hypothetical regime that combines the two.

## Numerical parity across build profiles

Both Release and Debug ran `powered-vacuum` for 18 ticks in `sql` mode. The selected persisted tables were:
`vehicle_physical_states`, `component_operational_states`, `energy_reservoir_states`, `thermal_node_states`, `component_reaction_wheel_states`.

Release hash: `64fed73bb6a3db2fda57f32fde983b5a0331941dabc8e75271784accb22da79d`.

Debug hash: `8e11a0fcdfc6817be7d4cbc0d3cb25a02be4d9ea05366caaaa2e53bb6c0b1bcd`.

The hashes are NOT identical. Direct SQLite cell comparisons found six floating-point differences in vehicle position/velocity and one in stored battery energy, with matching row counts. Largest position component difference: 0.52099609375 m, where inertial positions are of order 10^12 m. Largest velocity component difference: approximately 7.14e-8 m/s. Stored energy difference: approximately 2.76e-10 J. All 12 thermal rows and both operational state rows matched exactly. These differences alone do not establish a regression, but no bit-exact cross-build assertion or long-horizon error bound is justified.

## Boundary to Qt

`FlightController` uses a 5 ms polling timer for physics-step requests, with a pending-step gate and 1/60 s simulation timestep, and a separate approximately 16 ms presentation timer with interpolation. It treats reference-body changes specially. The `RocketconWorker` publishes profiling measurements for `service_->step()` and subsequent telemetry processing. Forecast work runs on another thread and previews are requested roughly every 125 ms under thrust and every 350 ms otherwise, plus special events. The existing `tests/real_flight_probe.cpp` can observe step and snapshot timings and pending steps.

No live Qt frame pacing, render duration, FFI transfer latency, or forecast worker contention was measured in this checkpoint. The existence of these controls does not establish absence of flicker or stutter.

## Decision and next target

1. Preserve DOP853 tolerances, ephemeris timestamps, N-body precision and all trajectory/Qt contracts. Release vacuum N-body cost is comparatively small.
2. Next investigate atmospheric initial/final aerodynamic diagnostics and their dependent reads/calculations, keeping distinct current/final epochs and the post-SOI reference context. Avoid merging samples incorrectly.
3. Align a future controlled benchmark with the actual Qt timestep (1/60 s), then use existing UI profiling signals to measure real Qt step, telemetry and frame pacing separately, before claiming an end-to-end 60 FPS result.
4. Treat the Release/Debug floating-point differences as documented and evaluate long-horizon numerical tolerances if future physical changes touch the integrator.

No commits or pushes were created.
