use runtime_abi::{
    INPUT_BOOL, INPUT_DOMAIN_BOOL, INPUT_DOMAIN_INT, INPUT_DOMAIN_STRING, INPUT_EMAIL, INPUT_IMAGE,
    INPUT_INT, INPUT_SLUG, INPUT_STRING, INPUT_UPLOAD, INPUT_URL, RequestValue, VelranInputValue,
};

fn string_wire(tag: u32, aux: u32, value: &str) -> VelranInputValue {
    VelranInputValue {
        tag,
        aux,
        payload: 0,
        data: value.as_ptr(),
        len: value.len() as u64,
    }
}

pub(super) fn wire(value: &RequestValue<'_>) -> VelranInputValue {
    match value {
        RequestValue::Int(v) => VelranInputValue {
            tag: INPUT_INT,
            aux: 0,
            payload: *v as u64,
            data: std::ptr::null(),
            len: 0,
        },
        RequestValue::Bool(v) => VelranInputValue {
            tag: INPUT_BOOL,
            aux: 0,
            payload: u64::from(*v),
            data: std::ptr::null(),
            len: 0,
        },
        RequestValue::String(v) => string_wire(INPUT_STRING, 0, v),
        RequestValue::Email(v) => string_wire(INPUT_EMAIL, 0, v),
        RequestValue::Url(v) => string_wire(INPUT_URL, 0, v),
        RequestValue::Slug(v) => string_wire(INPUT_SLUG, 0, v),
        RequestValue::DomainInt { domain, value } => VelranInputValue {
            tag: INPUT_DOMAIN_INT,
            aux: u32::from(*domain),
            payload: *value as u64,
            data: std::ptr::null(),
            len: 0,
        },
        RequestValue::DomainBool { domain, value } => VelranInputValue {
            tag: INPUT_DOMAIN_BOOL,
            aux: u32::from(*domain),
            payload: u64::from(*value),
            data: std::ptr::null(),
            len: 0,
        },
        RequestValue::DomainString { domain, value } => {
            string_wire(INPUT_DOMAIN_STRING, u32::from(*domain), value)
        }
        RequestValue::Upload(v) => VelranInputValue {
            tag: INPUT_UPLOAD,
            aux: 0,
            payload: 0,
            data: v.as_ptr(),
            len: v.len() as u64,
        },
        RequestValue::Image(v) => VelranInputValue {
            tag: INPUT_IMAGE,
            aux: 0,
            payload: 0,
            data: v.as_ptr(),
            len: v.len() as u64,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_tags_keep_type_identity() {
        let email = wire(&RequestValue::Email("user@example.com"));
        let slug = wire(&RequestValue::Slug("article-1"));
        let domain = wire(&RequestValue::DomainInt {
            domain: 7,
            value: 42,
        });
        assert_eq!(email.tag, INPUT_EMAIL);
        assert_eq!(slug.tag, INPUT_SLUG);
        assert_eq!(domain.tag, INPUT_DOMAIN_INT);
        assert_eq!(domain.aux, 7);
    }
}
