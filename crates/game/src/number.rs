//! Large-number formatting, ported from `src/utils/numberUtils.ts`.
//!
//! Forces en-US grouping (commas) and K/M/B/T… suffixes, and caps the
//! displayed mantissa so absurd magnitudes never fall back to scientific
//! notation.

const SUFFIXES: [&str; 12] = [
    "", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc",
];

/// `roundTo` from numberUtils.ts.
pub fn round_to(value: f64, decimals: usize) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let factor = 10_f64.powi(decimals as i32);
    (value * factor).round() / factor
}

/// Comma-group a digit string read left-to-right (e.g. "1234567" -> "1,234,567").
fn group_thousands(digits: &str) -> String {
    let n = digits.len();
    let mut out = String::with_capacity(n + n / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (n - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Round to an integer and comma-group (no decimals).
fn format_grouped_integer(value: f64) -> String {
    let neg = value < 0.0;
    let v = value.abs().round();
    let body = group_thousands(&format!("{v:.0}"));
    if neg {
        format!("-{body}")
    } else {
        body
    }
}

/// Round to `decimals` decimal places and group the integer part, always
/// emitting exactly `decimals` fraction digits (matches JS `toLocaleString`
/// with min == max fraction digits).
fn format_fixed(value: f64, decimals: usize) -> String {
    let neg = value < 0.0;
    let v = value.abs();
    let factor = 10_f64.powi(decimals as i32);
    let scaled = (v * factor).round();
    let int_part = (scaled / factor).floor();
    let frac = (scaled - int_part * factor).round() as u64;
    let int_str = group_thousands(&format!("{int_part:.0}"));
    let body = format!("{int_str}.{frac:0width$}", width = decimals);
    if neg {
        format!("-{body}")
    } else {
        body
    }
}

/// `formatNumber` from numberUtils.ts (default 1 decimal for suffixes).
pub fn format_number(value: f64, decimals: usize) -> String {
    if !value.is_finite() || value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    if abs < 1000.0 {
        return format!("{}", round_to(value, decimals));
    }
    if abs < 1_000_000.0 {
        return format_grouped_integer(value);
    }
    let exponent = ((abs.log10() / 3.0).floor() as usize).min(SUFFIXES.len() - 1);
    let scaled = value / 10_f64.powi((exponent * 3) as i32);
    let display_scaled = scaled.min(999_999.0);
    let plus = if scaled > display_scaled { "+" } else { "" };
    format!(
        "{}{}{}",
        format_fixed(display_scaled, decimals),
        SUFFIXES[exponent],
        plus
    )
}

/// `formatNumberWithPrecision` from numberUtils.ts (full number below 1e12,
/// then 3-decimal suffixes).
pub fn format_number_with_precision(value: f64) -> String {
    if !value.is_finite() || value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    if abs < 1_000_000_000_000.0 {
        return format_grouped_integer(value);
    }
    let exponent = ((abs.log10() / 3.0).floor() as usize).min(SUFFIXES.len() - 1);
    let scaled = value / 10_f64.powi((exponent * 3) as i32);
    let display_scaled = scaled.min(999_999.0);
    let plus = if scaled > display_scaled { "+" } else { "" };
    format!(
        "{}{}{}",
        format_fixed(display_scaled, 3),
        SUFFIXES[exponent],
        plus
    )
}

/// `formatDuration` from numberUtils.ts (e.g. "5h 30m 10s").
pub fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0s".to_string();
    }
    if seconds < 60.0 {
        return format!("{}s", seconds.floor());
    }
    let hours = (seconds / 3600.0).floor();
    let minutes = ((seconds % 3600.0) / 60.0).floor();
    let remaining = (seconds % 60.0).floor();
    let mut result = String::new();
    if hours > 0.0 {
        result.push_str(&format!("{hours:.0}h "));
    }
    if minutes > 0.0 || hours > 0.0 {
        result.push_str(&format!("{minutes:.0}m "));
    }
    result.push_str(&format!("{remaining:.0}s"));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_reference_values() {
        assert_eq!(format_number(999.0, 1), "999");
        assert_eq!(format_number(1000.0, 1), "1,000");
        assert_eq!(format_number(1_234_567.0, 1), "1.2M");
        assert_eq!(format_number(1_000_000.0, 1), "1.0M");
        assert_eq!(format_number(999_999.0, 1), "999,999");
    }

    #[test]
    fn format_precision_reference_values() {
        assert_eq!(format_number_with_precision(1_234_567.0), "1,234,567");
        assert_eq!(format_number_with_precision(1.5e12), "1.500T");
        assert_eq!(format_number_with_precision(0.0), "0");
    }

    #[test]
    fn non_finite_is_zero() {
        assert_eq!(format_number(f64::NAN, 1), "0");
        assert_eq!(format_number(f64::INFINITY, 1), "0");
    }

    #[test]
    fn format_duration_reference() {
        assert_eq!(format_duration(0.0), "0s");
        assert_eq!(format_duration(42.0), "42s");
        assert_eq!(format_duration(65.0), "1m 5s");
        assert_eq!(format_duration(3661.0), "1h 1m 1s");
    }
}
