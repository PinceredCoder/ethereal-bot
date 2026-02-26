pub const ORDER_DECIMALS: u128 = 1_000_000_000;
pub const ORDER_DECIMAL_PLACES: usize = ORDER_DECIMALS.ilog10() as usize;

pub fn format_order_decimal(raw: u128) -> String {
    let integer = raw / ORDER_DECIMALS;
    let fractional = raw % ORDER_DECIMALS;
    format!(
        "{integer}.{fractional:0width$}",
        width = ORDER_DECIMAL_PLACES
    )
}

#[cfg(test)]
mod tests {
    use super::{ORDER_DECIMAL_PLACES, ORDER_DECIMALS, format_order_decimal};

    #[test]
    fn decimal_places_are_derived_from_scale() {
        assert_eq!(ORDER_DECIMALS, 10u128.pow(ORDER_DECIMAL_PLACES as u32));
    }

    #[test]
    fn formats_whole_and_fractional_values() {
        assert_eq!(format_order_decimal(1_000_000_000), "1.000000000");
        assert_eq!(format_order_decimal(1_000_000_001), "1.000000001");
    }

    #[test]
    fn formats_subunit_values() {
        assert_eq!(format_order_decimal(1), "0.000000001");
    }
}
