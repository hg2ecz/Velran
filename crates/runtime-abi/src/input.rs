#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestValue<'a> {
    Int(i64),
    Bool(bool),
    String(&'a str),
    Email(&'a str),
    Url(&'a str),
    Slug(&'a str),
    DomainInt { domain: u16, value: i64 },
    DomainBool { domain: u16, value: bool },
    DomainString { domain: u16, value: &'a str },
    Upload(&'a [u8]),
    Image(&'a [u8]),
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VelranInputValue {
    pub tag: u32,
    pub aux: u32,
    pub payload: u64,
    pub data: *const u8,
    pub len: u64,
}

impl VelranInputValue {
    pub const EMPTY: Self = Self {
        tag: 0,
        aux: 0,
        payload: 0,
        data: std::ptr::null(),
        len: 0,
    };
}

pub const INPUT_INT: u32 = 1;
pub const INPUT_BOOL: u32 = 2;
pub const INPUT_STRING: u32 = 3;
pub const INPUT_EMAIL: u32 = 4;
pub const INPUT_URL: u32 = 5;
pub const INPUT_SLUG: u32 = 6;
pub const INPUT_DOMAIN_INT: u32 = 7;
pub const INPUT_DOMAIN_BOOL: u32 = 8;
pub const INPUT_DOMAIN_STRING: u32 = 9;
pub const INPUT_UPLOAD: u32 = 10;
pub const INPUT_IMAGE: u32 = 11;
pub const MAX_INPUT_FIELDS: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_layout_is_fixed_width() {
        assert_eq!(std::mem::size_of::<VelranInputValue>(), 32);
        assert_eq!(std::mem::align_of::<VelranInputValue>(), 8);
    }
}
