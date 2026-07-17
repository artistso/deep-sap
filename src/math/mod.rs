pub mod polynomial;
pub mod trig;

pub use polynomial::{
    bearing_rate_polynomial, fit_polynomial, least_squares_position, predict_polynomial,
    ssp_polynomial_leroy, tma_quadratic_prediction, Polynomial, PolynomialFit,
};
pub use trig::{
    angular_distance_abs_deg, bearing_least_squares_fix, bearing_to_vector, cross_fix_2_bearings,
    haversine_distance_km, law_of_cosines_distance, tma_bearing_only_tracking, trig_triangulation,
    vector_to_bearing, Bearing, BearingLineObservation, Covariance2D, FixEstimate, Position2D,
};
