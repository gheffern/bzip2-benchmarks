//! Statistical analysis module for benchmark timing samples with robust MAD dispersion.

use serde::{Deserialize, Serialize};

/// Statistical summary of benchmark timing samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    /// Median throughput in MB/s.
    pub median: f64,
    /// Arithmetic mean throughput in MB/s.
    pub mean: f64,
    /// Minimum recorded throughput in MB/s.
    pub min: f64,
    /// Maximum recorded throughput in MB/s.
    pub max: f64,
    /// Relative Standard Deviation percentage (RSD %).
    pub rsd_pct: f64,
    /// Median Absolute Deviation percentage (MAD %).
    pub mad_pct: f64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            median: 0.0,
            mean: 0.0,
            min: 0.0,
            max: 0.0,
            rsd_pct: 0.0,
            mad_pct: 0.0,
        }
    }
}

/// Compute statistical metrics from raw throughput samples (MB/s).
pub fn compute_stats(mut samples: Vec<f64>) -> Stats {
    if samples.is_empty() {
        return Stats::default();
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = samples.len();

    let median = if n % 2 == 0 {
        (samples[n / 2 - 1] + samples[n / 2]) / 2.0
    } else {
        samples[n / 2]
    };

    let min = samples[0];
    let max = samples[n - 1];
    let mean = samples.iter().sum::<f64>() / n as f64;

    let variance = samples
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / (if n > 1 { n - 1 } else { 1 }) as f64;

    let std_dev = variance.sqrt();
    let rsd_pct = if mean > 0.0 {
        (std_dev / mean) * 100.0
    } else {
        0.0
    };

    // Calculate Median Absolute Deviation (MAD)
    let mut abs_devs: Vec<f64> = samples.iter().map(|&x| (x - median).abs()).collect();
    abs_devs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad = if n.is_multiple_of(2) {
        (abs_devs[n / 2 - 1] + abs_devs[n / 2]) / 2.0
    } else {
        abs_devs[n / 2]
    };
    let mad_pct = if median > 0.0 {
        (1.4826 * mad / median) * 100.0
    } else {
        0.0
    };

    Stats {
        median,
        mean,
        min,
        max,
        rsd_pct,
        mad_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_stats_odd() {
        let samples = vec![100.0, 200.0, 300.0];
        let stats = compute_stats(samples);
        assert_eq!(stats.median, 200.0);
        assert_eq!(stats.min, 100.0);
        assert_eq!(stats.max, 300.0);
        assert_eq!(stats.mean, 200.0);
        assert!(stats.mad_pct > 0.0);
    }

    #[test]
    fn test_compute_stats_even() {
        let samples = vec![100.0, 200.0, 300.0, 400.0];
        let stats = compute_stats(samples);
        assert_eq!(stats.median, 250.0);
        assert_eq!(stats.min, 100.0);
        assert_eq!(stats.max, 400.0);
    }
}
