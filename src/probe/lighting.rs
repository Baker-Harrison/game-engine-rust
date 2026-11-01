use super::aov::Aov;
use super::targets::Targets;
use super::{Finding, Severity};

pub const HISTOGRAM_BINS: usize = 8;

pub struct LightingStats {
    pub covered_fraction: f32,
    pub histogram: [f32; HISTOGRAM_BINS],
    pub clipped: f32,
    pub crushed: f32,
    pub low_percentile: f32,
    pub high_percentile: f32,
    pub mean: f32,
}

pub fn analyze(aov: &Aov) -> LightingStats {
    let mut values: Vec<f32> = (0..aov.width * aov.height).filter(|&i| aov.covered(i)).map(|i| aov.luminance[i]).collect();
    values.sort_by(|a, b| a.total_cmp(b));
    let covered = values.len().max(1) as f32;
    let mut histogram = [0.0; HISTOGRAM_BINS];
    for value in &values {
        let bin = ((value * HISTOGRAM_BINS as f32) as usize).min(HISTOGRAM_BINS - 1);
        histogram[bin] += 1.0 / covered;
    }
    let percentile = |fraction: f32| values.get(((values.len() as f32 - 1.0) * fraction) as usize).copied().unwrap_or(0.0);
    LightingStats {
        covered_fraction: values.len() as f32 / (aov.width * aov.height) as f32,
        histogram,
        clipped: values.iter().filter(|&&v| v > 0.98).count() as f32 / covered,
        crushed: values.iter().filter(|&&v| v < 0.03).count() as f32 / covered,
        low_percentile: percentile(0.05),
        high_percentile: percentile(0.95),
        mean: values.iter().sum::<f32>() / covered,
    }
}

pub fn findings(view: &str, stats: &LightingStats, targets: &Targets) -> Vec<Finding> {
    let mut out = Vec::new();
    let subject = format!("view {}", view);
    if stats.covered_fraction < 0.02 {
        return out;
    }
    if stats.clipped > targets.clipped_max {
        out.push(Finding::new(
            Severity::Warn,
            &subject,
            "clipped_fraction",
            stats.clipped,
            format!("<= {:.2}", targets.clipped_max),
            "lower albedo or light intensity; pure white loses form",
        ));
    }
    if stats.crushed > targets.crushed_max {
        out.push(Finding::new(
            Severity::Warn,
            &subject,
            "crushed_fraction",
            stats.crushed,
            format!("<= {:.2}", targets.crushed_max),
            "raise ambient or fill light; shadows should keep detail",
        ));
    }
    let contrast = stats.high_percentile - stats.low_percentile;
    if contrast < targets.contrast_min {
        out.push(Finding::new(
            Severity::Warn,
            &subject,
            "contrast_p95_minus_p5",
            contrast,
            format!(">= {:.2}", targets.contrast_min),
            "flat lighting; add a stronger key light direction or darker materials in shade",
        ));
    }
    out
}

pub fn histogram_line(stats: &LightingStats) -> String {
    stats.histogram.iter().map(|h| format!("{:.2}", h)).collect::<Vec<_>>().join(" ")
}
