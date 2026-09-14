use super::result::VelranResult;
use crate::{STATUS_OK, VALUE_BOOL};

#[test]
fn result_layout_is_fixed_width() {
    assert_eq!(std::mem::size_of::<VelranResult>(), 40);
    assert_eq!(std::mem::align_of::<VelranResult>(), 8);
}

#[test]
fn scalar_payload_round_trips() {
    assert_eq!(VelranResult::int(-42, 7).payload as i64, -42);
    assert_eq!(VelranResult::bool(true, 3).fuel_used, 3);
}

#[test]
fn malformed_bool_is_rejected() {
    assert!(
        !VelranResult {
            status: STATUS_OK,
            value_tag: VALUE_BOOL,
            payload: 2,
            fuel_used: 1,
            allocated_bytes: 0,
            output_len: 0,
        }
        .is_well_formed()
    );
}
