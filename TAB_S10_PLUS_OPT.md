# Galaxy Tab S10+ Validation Plan

## Input

Test Pointer Events for pen detection, contact pressure, tilt, twist, hover, barrel button, coalesced events, palm rejection behavior, and touch fallback. Record unsupported fields as zero/default rather than inferring values.

## Rendering

Measure—not assume—frame pacing at 60, 90, and 120 Hz. Use separate fixed simulation and render rates, adaptive waterfall resolution, and a thermal fallback tier. Report median, p95, and p99 frame time during a ten-minute mission.

## Layout

Prioritize landscape use, minimum 44 CSS-pixel touch targets, readable track tables, safe-area insets, and preservation of the central playfield when panels open.

## Browser lifecycle

Verify installation, offline launch, service-worker updates, wake-lock reacquisition, visibility changes, orientation changes, and recovery after the browser discards the tab.

## Native option

A later Kotlin shell may provide richer Android `MotionEvent`, lifecycle, audio, and low-latency rendering integration while retaining Rust for deterministic simulation and mathematics. That should be a measured architectural decision, not a prerequisite for the current web vertical slice.
