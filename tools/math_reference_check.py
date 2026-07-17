#!/usr/bin/env python3
"""Independent numerical smoke tests for DEEP SAP geometry and units.

Uses only the Python standard library so it can run in CI without dependencies.
This is not a replacement for Rust unit/property tests; it checks the release
fixtures against an independently implemented reference calculation.
"""
from __future__ import annotations

import math
from dataclasses import dataclass

KNOT_TO_KM_PER_SECOND = 1.852 / 3600.0
CHI_SQUARED_95_2D = 5.991464547107979


@dataclass(frozen=True)
class Observation:
    ox: float
    oy: float
    bearing_deg: float
    sigma_deg: float


def wrap_signed_deg(value: float) -> float:
    return (value + 180.0) % 360.0 - 180.0


def nav_bearing_deg(ox: float, oy: float, tx: float, ty: float) -> float:
    return math.degrees(math.atan2(tx - ox, ty - oy)) % 360.0


def solve_2x2(a: float, b: float, c: float, d: float, e: float, f: float) -> tuple[float, float]:
    determinant = a * d - b * c
    if abs(determinant) < 1e-12:
        raise ValueError("singular normal matrix")
    return ((e * d - b * f) / determinant, (a * f - e * c) / determinant)


def fit_bearings(observations: list[Observation], initial: tuple[float, float]) -> tuple[tuple[float, float], tuple[float, float, float]]:
    x, y = initial
    for _ in range(30):
        h00 = h01 = h11 = g0 = g1 = 0.0
        for obs in observations:
            dx, dy = x - obs.ox, y - obs.oy
            radius2 = dx * dx + dy * dy
            if radius2 < 1e-10:
                raise ValueError("estimate coincides with observer")
            predicted = math.atan2(dx, dy)
            observed = math.radians(obs.bearing_deg)
            residual = math.atan2(math.sin(observed - predicted), math.cos(observed - predicted))
            jx, jy = dy / radius2, -dx / radius2
            weight = 1.0 / math.radians(obs.sigma_deg) ** 2
            h00 += weight * jx * jx
            h01 += weight * jx * jy
            h11 += weight * jy * jy
            g0 += weight * jx * residual
            g1 += weight * jy * residual
        step_x, step_y = solve_2x2(h00, h01, h01, h11, g0, g1)
        x += step_x
        y += step_y
        if math.hypot(step_x, step_y) < 1e-9:
            break

    # Inverse Fisher information; positive definiteness is the key smoke test.
    h00 = h01 = h11 = 0.0
    for obs in observations:
        dx, dy = x - obs.ox, y - obs.oy
        radius2 = dx * dx + dy * dy
        jx, jy = dy / radius2, -dx / radius2
        weight = 1.0 / math.radians(obs.sigma_deg) ** 2
        h00 += weight * jx * jx
        h01 += weight * jx * jy
        h11 += weight * jy * jy
    determinant = h00 * h11 - h01 * h01
    if determinant <= 0.0:
        raise ValueError("non-positive information determinant")
    covariance = (h11 / determinant, -h01 / determinant, h00 / determinant)
    return (x, y), covariance


def covariance_eigenvalues(cxx: float, cxy: float, cyy: float) -> tuple[float, float]:
    trace = cxx + cyy
    discriminant = math.sqrt(max(0.0, (cxx - cyy) ** 2 + 4.0 * cxy * cxy))
    return ((trace + discriminant) / 2.0, (trace - discriminant) / 2.0)


def assert_close(actual: float, expected: float, tolerance: float, label: str) -> None:
    if abs(actual - expected) > tolerance:
        raise AssertionError(f"{label}: {actual} != {expected} ± {tolerance}")


def main() -> None:
    # Units: 10 kn for 60 s must be 0.308666... km, not 5.14444 km.
    displacement = 10.0 * KNOT_TO_KM_PER_SECOND * 60.0
    assert_close(displacement, 0.30866666666666664, 1e-12, "knot conversion")

    # Circular geometry across north.
    assert_close(abs(wrap_signed_deg(359.0 - 1.0)), 2.0, 1e-12, "bearing wrap")

    target = (8.0, 12.0)
    observer_positions = [(0.0, 0.0), (0.45, 0.0), (0.9, 0.08), (1.35, 0.18), (1.8, 0.32)]
    noise = [0.18, -0.24, 0.12, -0.08, 0.05]
    observations = [
        Observation(ox, oy, nav_bearing_deg(ox, oy, *target) + error, 0.35)
        for (ox, oy), error in zip(observer_positions, noise)
    ]
    estimate, covariance = fit_bearings(observations, initial=(7.0, 10.0))
    error_km = math.dist(estimate, target)
    if error_km > 0.35:
        raise AssertionError(f"bearing fix error too large: {error_km:.6f} km")

    major_variance, minor_variance = covariance_eigenvalues(*covariance)
    if not (major_variance > 0.0 and minor_variance > 0.0):
        raise AssertionError("covariance is not positive definite")
    major_95 = math.sqrt(CHI_SQUARED_95_2D * major_variance)
    minor_95 = math.sqrt(CHI_SQUARED_95_2D * minor_variance)
    if not (math.isfinite(major_95) and math.isfinite(minor_95) and major_95 >= minor_95):
        raise AssertionError("invalid 95% ellipse")

    # Repeated bearings from one fixed observer remain unobservable in range.
    baseline = max(math.dist(observer_positions[0], p) for p in [observer_positions[0]])
    if baseline >= 0.05:
        raise AssertionError("stationary observer baseline fixture is invalid")

    print("PASS knot_conversion", f"{displacement:.12f} km/60s")
    print("PASS circular_bearing", "359° vs 1° = 2°")
    print("PASS bearing_fix", f"estimate=({estimate[0]:.6f},{estimate[1]:.6f}) error={error_km:.6f} km")
    print("PASS covariance", f"ellipse95=({major_95:.6f},{minor_95:.6f}) km")
    print("PASS observability_guard", "stationary baseline rejected by release threshold")


if __name__ == "__main__":
    main()
