pub fn convert_storage_unit(bytes: usize) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];

    let mut result: f64 = bytes as f64;
    let mut unit_index = 0;

    while result >= 1024.0 && unit_index < UNITS.len() - 1 {
        result /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2}{}", result, UNITS[unit_index])
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_convert_storage_unit() {
        assert_eq!(convert_storage_unit(1029384756), "981.70MiB");
    }
}
