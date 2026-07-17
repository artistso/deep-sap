//! Numerically stable polynomial utilities.
//! Fits are performed in normalized time with f64 and QR decomposition.

#[derive(Debug, Clone)]
pub struct Polynomial {
    pub coeffs: Vec<f64>,
    pub x_offset: f64,
    pub x_scale: f64,
}

impl Polynomial {
    pub fn new(coeffs: Vec<f64>) -> Self {
        Self {
            coeffs,
            x_offset: 0.0,
            x_scale: 1.0,
        }
    }

    pub fn with_normalization(coeffs: Vec<f64>, x_offset: f64, x_scale: f64) -> Self {
        Self {
            coeffs,
            x_offset,
            x_scale: x_scale.max(f64::EPSILON),
        }
    }

    pub fn degree(&self) -> usize {
        self.coeffs.len().saturating_sub(1)
    }

    pub fn eval(&self, x: f64) -> f64 {
        let normalized_x = (x - self.x_offset) / self.x_scale;
        self.coeffs
            .iter()
            .rev()
            .fold(0.0, |result, coefficient| {
                result * normalized_x + coefficient
            })
    }

    pub fn derivative_at(&self, x: f64) -> f64 {
        if self.coeffs.len() <= 1 {
            return 0.0;
        }
        let normalized_x = (x - self.x_offset) / self.x_scale;
        let derivative_normalized = self
            .coeffs
            .iter()
            .enumerate()
            .skip(1)
            .rev()
            .fold(0.0, |result, (power, coefficient)| {
                result * normalized_x + *coefficient * power as f64
            });
        derivative_normalized / self.x_scale
    }
}

impl std::fmt::Display for Polynomial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let terms = self
            .coeffs
            .iter()
            .enumerate()
            .map(|(power, coefficient)| {
                if power == 0 {
                    format!("{coefficient:.6}")
                } else {
                    format!("{coefficient:+.6}*z^{power}")
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        write!(
            formatter,
            "{terms}, z=(x-{:.6})/{:.6}",
            self.x_offset, self.x_scale
        )
    }
}

#[derive(Debug, Clone)]
pub struct PolynomialFit {
    pub poly: Polynomial,
    pub r_squared: f64,
    pub rmse: f64,
}

/// Least-squares polynomial fit using normalized coordinates and modified
/// Gram-Schmidt QR. This avoids the catastrophic conditioning of raw-time
/// normal equations.
pub fn fit_polynomial(points: &[(f64, f64)], degree: usize) -> Option<PolynomialFit> {
    if points.len() <= degree || degree > 6 {
        return None;
    }

    let count = points.len();
    let columns = degree + 1;
    let x_offset = points.iter().map(|(x, _)| *x).sum::<f64>() / count as f64;
    let x_scale = points
        .iter()
        .map(|(x, _)| (x - x_offset).abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let normalized_x: Vec<f64> = points
        .iter()
        .map(|(x, _)| (x - x_offset) / x_scale)
        .collect();
    let y: Vec<f64> = points.iter().map(|(_, y)| *y).collect();

    let mut q_columns = vec![vec![0.0; count]; columns];
    let mut r = vec![vec![0.0; columns]; columns];

    for column in 0..columns {
        let mut vector: Vec<f64> = normalized_x
            .iter()
            .map(|x| x.powi(column as i32))
            .collect();

        for previous in 0..column {
            let projection = dot(&q_columns[previous], &vector);
            r[previous][column] = projection;
            for row in 0..count {
                vector[row] -= projection * q_columns[previous][row];
            }
        }

        let norm = dot(&vector, &vector).sqrt();
        if norm < 1.0e-12 {
            return None;
        }
        r[column][column] = norm;
        for row in 0..count {
            q_columns[column][row] = vector[row] / norm;
        }
    }

    let q_transpose_y: Vec<f64> = q_columns.iter().map(|column| dot(column, &y)).collect();
    let coeffs = back_substitute_upper_triangular(&r, &q_transpose_y)?;
    let poly = Polynomial::with_normalization(coeffs, x_offset, x_scale);

    let mean_y = y.iter().sum::<f64>() / count as f64;
    let mut residual_sum_squares = 0.0;
    let mut total_sum_squares = 0.0;
    for (x, observed_y) in points {
        let residual = observed_y - poly.eval(*x);
        residual_sum_squares += residual * residual;
        let centered = observed_y - mean_y;
        total_sum_squares += centered * centered;
    }

    let r_squared = if total_sum_squares < 1.0e-12 {
        1.0
    } else {
        1.0 - residual_sum_squares / total_sum_squares
    };
    let rmse = (residual_sum_squares / count as f64).sqrt();

    Some(PolynomialFit {
        poly,
        r_squared,
        rmse,
    })
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn back_substitute_upper_triangular(
    matrix: &[Vec<f64>],
    right_hand_side: &[f64],
) -> Option<Vec<f64>> {
    let size = right_hand_side.len();
    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        if matrix[row][row].abs() < 1.0e-12 {
            return None;
        }
        let known_sum = ((row + 1)..size)
            .map(|column| matrix[row][column] * solution[column])
            .sum::<f64>();
        solution[row] = (right_hand_side[row] - known_sum) / matrix[row][row];
    }
    Some(solution)
}

pub fn predict_polynomial(fit: &PolynomialFit, x: f64) -> f64 {
    fit.poly.eval(x)
}

/// Simplified Leroy-style sound-speed approximation used by the simulation.
pub fn ssp_polynomial_leroy(temp_c: f32, salinity_psu: f32, depth_m: f32) -> f32 {
    let temperature = temp_c;
    1402.5
        + 5.0 * temperature
        - 0.055 * temperature * temperature
        + 0.00029 * temperature * temperature * temperature
        + (1.34 - 0.01 * temperature) * (salinity_psu - 35.0)
        + 0.016 * depth_m
}

pub fn ssp_polynomial_coeffs_for_reference_ocean() -> Polynomial {
    let points: Vec<(f64, f64)> = (0..=100)
        .step_by(10)
        .map(|depth| {
            (
                depth as f64,
                ssp_polynomial_leroy(11.8, 32.5, depth as f32) as f64,
            )
        })
        .collect();
    fit_polynomial(&points, 2)
        .map(|fit| fit.poly)
        .unwrap_or_else(|| Polynomial::new(vec![1480.0, 0.016]))
}

/// Position prediction fitted only from estimated track positions.
pub fn tma_quadratic_prediction(
    history: &[(f64, f64, f64)],
    future_time_s: f64,
) -> Option<(f64, f64)> {
    if history.len() < 3 {
        return None;
    }
    let x_points: Vec<(f64, f64)> = history
        .iter()
        .map(|(time, x, _)| (*time, *x))
        .collect();
    let y_points: Vec<(f64, f64)> = history
        .iter()
        .map(|(time, _, y)| (*time, *y))
        .collect();
    let fit_x = fit_polynomial(&x_points, 2)?;
    let fit_y = fit_polynomial(&y_points, 2)?;
    Some((
        fit_x.poly.eval(future_time_s),
        fit_y.poly.eval(future_time_s),
    ))
}

pub fn least_squares_position(satellite_observations: &[(f64, f64, f64, f64)]) -> Option<(f64, f64)> {
    if satellite_observations.len() < 2 {
        return None;
    }
    let mut x = satellite_observations
        .iter()
        .map(|(satellite_x, _, _, _)| satellite_x)
        .sum::<f64>()
        / satellite_observations.len() as f64;
    let mut y = satellite_observations
        .iter()
        .map(|(_, satellite_y, _, _)| satellite_y)
        .sum::<f64>()
        / satellite_observations.len() as f64;

    for _ in 0..200 {
        let mut gradient_x = 0.0;
        let mut gradient_y = 0.0;
        for (satellite_x, satellite_y, _, range) in satellite_observations {
            let dx = x - satellite_x;
            let dy = y - satellite_y;
            let distance = dx.hypot(dy).max(0.001);
            let residual = distance - range;
            gradient_x += residual * dx / distance;
            gradient_y += residual * dy / distance;
        }
        let scale = 0.05 / satellite_observations.len() as f64;
        x -= gradient_x * scale;
        y -= gradient_y * scale;
    }
    Some((x, y))
}

pub fn bearing_rate_polynomial(bearings: &[(f64, f64)]) -> Option<PolynomialFit> {
    fit_polynomial(bearings, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polynomial_evaluation_uses_horner() {
        let polynomial = Polynomial::new(vec![1.0, 2.0, 3.0]);
        assert!((polynomial.eval(2.0) - 17.0).abs() < 1.0e-12);
    }

    #[test]
    fn linear_fit_is_exact() {
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let fit = fit_polynomial(&points, 1).expect("fit");
        assert!(fit.r_squared > 0.999_999);
        assert!((fit.poly.eval(5.0) - 5.0).abs() < 1.0e-9);
    }

    #[test]
    fn large_timestamps_remain_stable() {
        let points: Vec<(f64, f64)> = (0..30)
            .map(|index| {
                let time = 3_600.0 + index as f64;
                let elapsed = time - 3_600.0;
                (time, 5.0 + 0.02 * elapsed + 0.001 * elapsed * elapsed)
            })
            .collect();
        let fit = fit_polynomial(&points, 2).expect("stable normalized fit");
        let predicted = fit.poly.eval(3_640.0);
        let expected = 5.0 + 0.02 * 40.0 + 0.001 * 40.0 * 40.0;
        assert!((predicted - expected).abs() < 1.0e-7);
    }
}
