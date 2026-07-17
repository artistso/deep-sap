# DEEP SAP — Vertical-Slice Game Specification

## Product definition

A tablet-first synthetic sensor-analysis game. The player studies noisy bearing and tonal evidence, creates and manages track hypotheses, and receives an after-action comparison against hidden truth only after the mission.

## Non-goals

- Real-world target detection or identification
- Operational anti-submarine warfare capability
- Navigation, maritime safety, surveillance, or targeting
- Classified or restricted acoustic libraries
- Inferring game targets from public satellite, AIS, buoy, or weather data

## Core loop

1. Observe a time-varying bearing display and tonal waterfall.
2. Mark evidence using pen, touch, or mouse.
3. Associate evidence with an existing tentative track or start a new track.
4. Accumulate observer baseline and measurement diversity.
5. Review estimated position, covariance ellipse, confidence, and track age.
6. Classify the synthetic contact from observable evidence.
7. Complete the mission and reveal truth in an after-action report.

## Track lifecycle

```text
Tentative -> Confirmed -> Coasting -> Dropped
```

A track becomes confirmed only after independent observations satisfy minimum count, time span, observer baseline, residual, and covariance thresholds. Classification confidence is separate from localization quality.

## Simulation model

- Truth state is inaccessible to tracking and UI systems during play.
- The sensor layer emits noisy observations with no target ID, class, or faction.
- Motion uses kilometres, seconds, knots, and explicit conversion constants.
- Bearing localization uses weighted nonlinear least squares.
- Uncertainty is derived from covariance and rendered as a 95% ellipse.
- Public environmental values alter noise/propagation parameters only.

## Input model

- Pen hover: inspect
- Pen contact: mark or select
- Pressure: evidence aperture/emphasis
- Barrel button: confirm
- Eraser action when available: delete mark
- One-finger drag: pan
- Two-finger gesture: zoom
- Mouse/touch: complete fallback controls

## Vertical-slice completion criteria

- One deterministic 8–12 minute mission
- At least three synthetic contact types and false alarms
- Player-created marks and track assignments
- Confirm/coast/drop lifecycle
- Texture-backed waterfall
- Pause/resume and offline operation
- Replay with truth reveal
- Automated deterministic integration test
- Measured target-tablet performance and thermal profile
