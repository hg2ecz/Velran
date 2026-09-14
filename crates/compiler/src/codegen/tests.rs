use super::*;
#[test]
fn rust_identifier_is_sanitized() {
    assert_eq!(rust_ident("admin.orders-list"), "admin_orders_list");
    assert_eq!(rust_ident("42"), "_42");
}
#[test]
fn pure_function_identifier_is_snake_case_and_collision_resistant() {
    let ident = pure_function_ident("fft4096-x10000::fft4096");
    assert!(ident.starts_with("velran_pure_fft4096_x10000_fft4096_"));
    assert_ne!(pure_function_ident("Foo"), pure_function_ident("foo"));
}
#[test]
fn crate_identifier_is_snake_case_and_collapses_separators() {
    assert_eq!(
        crate_ident("Calculator__Calculator"),
        "calculator_calculator"
    );
    assert_eq!(
        crate_ident("catalog::calculate-product"),
        "catalog_calculate_product"
    );
}
