use crate::{STATUS_OK, STATUS_OUTPUT_TOO_SMALL, VALUE_BOOL, VALUE_HTML, VALUE_INT, VALUE_NONE};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VelranResult {
    pub status: u32,
    pub value_tag: u32,
    pub payload: u64,
    pub fuel_used: u64,
    pub allocated_bytes: u64,
    pub output_len: u64,
}

impl VelranResult {
    pub const fn none(status: u32, fuel_used: u64) -> Self {
        Self {
            status,
            value_tag: VALUE_NONE,
            payload: 0,
            fuel_used,
            allocated_bytes: 0,
            output_len: 0,
        }
    }
    pub const fn int(value: i64, fuel_used: u64) -> Self {
        Self {
            status: STATUS_OK,
            value_tag: VALUE_INT,
            payload: value as u64,
            fuel_used,
            allocated_bytes: 0,
            output_len: 0,
        }
    }
    pub const fn bool(value: bool, fuel_used: u64) -> Self {
        Self {
            status: STATUS_OK,
            value_tag: VALUE_BOOL,
            payload: value as u64,
            fuel_used,
            allocated_bytes: 0,
            output_len: 0,
        }
    }
    pub const fn is_well_formed(self) -> bool {
        match (self.status, self.value_tag) {
            (STATUS_OK, VALUE_INT) => self.output_len == 0,
            (STATUS_OK, VALUE_BOOL) => self.payload <= 1 && self.output_len == 0,
            (STATUS_OK, VALUE_NONE) => self.payload == 0 && self.output_len == 0,
            (STATUS_OK, VALUE_HTML) => self.payload == 0,
            (STATUS_OUTPUT_TOO_SMALL, VALUE_HTML) => self.payload == 0 && self.output_len > 0,
            (_, VALUE_NONE) => self.payload == 0 && self.output_len == 0,
            _ => false,
        }
    }
}
