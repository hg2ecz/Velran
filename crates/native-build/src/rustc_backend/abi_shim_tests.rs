use super::abi_shim::source;

#[test]
fn shim_is_the_only_export_boundary() {
    let shim = source();
    assert!(shim.contains("#[unsafe(no_mangle)]"));
    assert!(shim.contains("canonical_email"));
    assert!(shim.contains("canonical_slug"));
    assert!(shim.contains("DomainString"));
    assert!(!shim.contains("use crate::implementation;"));
    assert!(shim.contains("implementation::velran_invoke_safe"));
}
