use crate::gate16_graph::{GraphSignature, multiset_jaccard};
use crate::gate16_repr::ResampledTrajectory;
use crate::gate16_types::{EventToken, ProcessGeometryObject, TRAJECTORY_POINTS};
use wide::f32x8;

pub(crate) fn squared_l2_simd(left: &[f32], right: &[f32]) -> f64 {
    debug_assert_eq!(left.len(), right.len());
    let packed = left.len() / 8 * 8;
    let mut total = 0.0f64;
    for offset in (0..packed).step_by(8) {
        let left = f32x8::from(&left[offset..offset + 8]);
        let right = f32x8::from(&right[offset..offset + 8]);
        let delta = left - right;
        total += (delta * delta).reduce_add() as f64;
    }
    for index in packed..left.len() {
        total += (left[index] - right[index]).powi(2) as f64;
    }
    total
}

pub(crate) fn squared_l2_scalar(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| (left - right).powi(2) as f64)
        .sum()
}

pub(crate) fn robust_l1(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| (left - right).abs() as f64)
        .sum::<f64>()
        / left.len().max(1) as f64
}

pub(crate) fn robust_l2(left: &[f32], right: &[f32]) -> f64 {
    (squared_l2_simd(left, right) / left.len().max(1) as f64).sqrt()
}

pub(crate) fn mixed_gower(
    left: &[f32],
    right: &[f32],
    left_categories: &[i16],
    right_categories: &[i16],
) -> f64 {
    let continuous = left
        .iter()
        .zip(right)
        .map(|(&left, &right)| ((left - right).abs() / 12.0).min(1.0) as f64)
        .sum::<f64>();
    let categorical = left_categories
        .iter()
        .zip(right_categories)
        .filter(|(left, right)| left != right)
        .count() as f64;
    (continuous + categorical) / (left.len() + left_categories.len()).max(1) as f64
}

fn state_mismatch(left: &[i16], right: &[i16]) -> f64 {
    left.iter()
        .zip(right)
        .filter(|(left, right)| left != right)
        .count() as f64
        / left.len().max(1) as f64
}

pub(crate) fn trajectory_pointwise(left: &ResampledTrajectory, right: &ResampledTrajectory) -> f64 {
    robust_l2(&left.continuous, &right.continuous)
        + 0.25 * state_mismatch(&left.states, &right.states)
}

fn derivative(values: &[f32], points: usize) -> Vec<f32> {
    let channels = values.len() / points;
    let mut output = Vec::with_capacity((points - 1) * channels);
    for point in 1..points {
        let previous = (point - 1) * channels;
        let current = point * channels;
        for channel in 0..channels {
            output.push(values[current + channel] - values[previous + channel]);
        }
    }
    output
}

pub(crate) fn trajectory_derivative_aware(
    left: &ResampledTrajectory,
    right: &ResampledTrajectory,
) -> f64 {
    let levels = trajectory_pointwise(left, right);
    let left_derivative = derivative(&left.continuous, left.points);
    let right_derivative = derivative(&right.continuous, right.points);
    levels + 0.5 * robust_l2(&left_derivative, &right_derivative)
}

fn point_cost(
    left: &ResampledTrajectory,
    left_point: usize,
    right: &ResampledTrajectory,
    right_point: usize,
) -> f64 {
    let channels = left.continuous.len() / left.points;
    let left_values = &left.continuous[left_point * channels..(left_point + 1) * channels];
    let right_values = &right.continuous[right_point * channels..(right_point + 1) * channels];
    robust_l2(left_values, right_values)
        + 0.25 * f64::from(left.states[left_point] != right.states[right_point])
}

pub(crate) fn bounded_dtw(left: &ResampledTrajectory, right: &ResampledTrajectory) -> f64 {
    const WINDOW: usize = 2;
    let rows = left.points;
    let cols = right.points;
    let mut previous = vec![f64::INFINITY; cols + 1];
    let mut current = vec![f64::INFINITY; cols + 1];
    previous[0] = 0.0;
    for row in 1..=rows {
        current.fill(f64::INFINITY);
        let start = row.saturating_sub(WINDOW).max(1);
        let end = (row + WINDOW).min(cols);
        for column in start..=end {
            current[column] = point_cost(left, row - 1, right, column - 1)
                + previous[column]
                    .min(current[column - 1])
                    .min(previous[column - 1]);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[cols] / rows.max(cols) as f64
}

fn sample_at(
    object: &ProcessGeometryObject,
    canonical: bool,
    target: f32,
    values: &mut [f32],
) -> i16 {
    let samples = &object.trajectory;
    let index = samples.partition_point(|sample| sample.elapsed_bars as f32 <= target);
    let hi = index.min(samples.len() - 1);
    let lo = hi.saturating_sub(1);
    let lo_age = samples[lo].elapsed_bars as f32;
    let hi_age = samples[hi].elapsed_bars as f32;
    let weight = if hi_age > lo_age {
        ((target - lo_age) / (hi_age - lo_age)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let low = if canonical {
        &samples[lo].continuous_canonical
    } else {
        &samples[lo].continuous_raw
    };
    let high = if canonical {
        &samples[hi].continuous_canonical
    } else {
        &samples[hi].continuous_raw
    };
    for channel in 0..values.len() {
        values[channel] = low[channel] * (1.0 - weight) + high[channel] * weight;
    }
    let state_index = index.saturating_sub(1);
    if canonical {
        samples[state_index].state_canonical
    } else {
        samples[state_index].state_raw
    }
}

pub(crate) fn shared_prefix_pointwise(
    left: &ProcessGeometryObject,
    right: &ProcessGeometryObject,
    canonical: bool,
) -> Option<(f64, usize, u32, u32)> {
    if left.trajectory.len() < 4 || right.trajectory.len() < 4 {
        return None;
    }
    let horizon = left.observed_bars().min(right.observed_bars());
    if horizon < 3 {
        return None;
    }
    let channels = left.trajectory.first()?.continuous_raw.len();
    if channels != right.trajectory.first()?.continuous_raw.len() {
        return None;
    }
    let mut left_values = vec![0.0f32; channels];
    let mut right_values = vec![0.0f32; channels];
    let mut squared = 0.0;
    let mut state_difference = 0usize;
    for point in 0..TRAJECTORY_POINTS {
        let target = horizon as f32 * point as f32 / (TRAJECTORY_POINTS - 1) as f32;
        let left_state = sample_at(left, canonical, target, &mut left_values);
        let right_state = sample_at(right, canonical, target, &mut right_values);
        squared += squared_l2_simd(&left_values, &right_values);
        state_difference += usize::from(left_state != right_state);
    }
    let continuous = (squared / (TRAJECTORY_POINTS * channels) as f64).sqrt();
    let state = state_difference as f64 / TRAJECTORY_POINTS as f64;
    let seconds = left.observed_seconds().min(right.observed_seconds());
    Some((
        continuous + 0.25 * state,
        TRAJECTORY_POINTS,
        horizon,
        seconds,
    ))
}

fn token_substitution(left: &EventToken, right: &EventToken) -> f64 {
    let semantic = 0.75 * f64::from(left.event_code != right.event_code)
        + 0.25 * f64::from(left.direction != right.direction)
        + 0.25 * f64::from(left.state_code != right.state_code)
        + 0.25 * f64::from(left.terminal_reason_code != right.terminal_reason_code);
    let timing =
        ((left.delta_seconds as f64 + 1.0).ln() - (right.delta_seconds as f64 + 1.0).ln()).abs();
    semantic + 0.1 * timing
}

pub(crate) fn typed_edit(left: &[EventToken], right: &[EventToken]) -> f64 {
    let mut previous: Vec<_> = (0..=right.len()).map(|value| value as f64).collect();
    let mut current = vec![0.0; right.len() + 1];
    for (left_index, left_token) in left.iter().enumerate() {
        current[0] = (left_index + 1) as f64;
        for (right_index, right_token) in right.iter().enumerate() {
            current[right_index + 1] = (previous[right_index + 1] + 1.0)
                .min(current[right_index] + 1.0)
                .min(previous[right_index] + token_substitution(left_token, right_token));
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()] / left.len().max(right.len()).max(1) as f64
}

pub(crate) fn typed_graph_distance(left: &GraphSignature, right: &GraphSignature) -> f64 {
    multiset_jaccard(&left.typed_multiset, &right.typed_multiset)
}

pub(crate) fn wl2_graph_distance(left: &GraphSignature, right: &GraphSignature) -> f64 {
    multiset_jaccard(&left.wl2_multiset, &right.wl2_multiset)
}
