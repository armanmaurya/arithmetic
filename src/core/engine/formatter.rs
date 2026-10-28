pub fn format_number(val: f64) -> String {
    if val == 0.0 {
        return "0".into();
    }
    let abs = val.abs();
    if !(1e-7..1e12).contains(&abs) {
        return format!("{val:.6e}");
    }
    let rounded = (val * 1e10).round() / 1e10;
    if (rounded - rounded.trunc()).abs() < 1e-10 {
        format!("{}", rounded as i64)
    } else {
        let s = format!("{rounded:.10}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
