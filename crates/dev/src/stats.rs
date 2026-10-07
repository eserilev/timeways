//! The spread of a list of numbers, for the benches: the latency of a model, and the frame
//! rate of a phase. The percentiles match those of `/twdev fps` in the addon.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Spread {
    pub count: usize,
    pub min: f64,
    pub p5: f64,
    pub median: f64,
    pub p95: f64,
    pub max: f64,
    pub mean: f64,
}

/// The value at `fraction` of a sorted list, between its two nearest items. None for an
/// empty list.
#[must_use]
pub fn percentile(sorted: &[f64], fraction: f64) -> Option<f64> {
    let last = sorted.len().checked_sub(1)?;
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a list of samples is far shorter than 2^52, and the rank is in 0..=last"
    )]
    let (rank, low) = {
        let rank = fraction.clamp(0.0, 1.0) * last as f64;
        (rank, rank.floor() as usize)
    };
    let high = (low + 1).min(last);
    #[allow(clippy::cast_precision_loss, reason = "as above")]
    let weight = rank - low as f64;
    Some(sorted[low] + weight * (sorted[high] - sorted[low]))
}

/// The spread of the finite values. None when none is finite.
#[must_use]
pub fn spread(values: &[f64]) -> Option<Spread> {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    #[allow(clippy::cast_precision_loss, reason = "a list of samples is short")]
    let mean = sorted.iter().sum::<f64>() / count as f64;
    Some(Spread {
        count,
        min: *sorted.first()?,
        p5: percentile(&sorted, 0.05)?,
        median: percentile(&sorted, 0.5)?,
        p95: percentile(&sorted, 0.95)?,
        max: *sorted.last()?,
        mean,
    })
}

/// How much lower `now` is than `before`, in percent. None for a `before` of 0.
#[must_use]
pub fn drop_percent(before: f64, now: f64) -> Option<f64> {
    (before > 0.0).then(|| (before - now) / before * 100.0)
}
