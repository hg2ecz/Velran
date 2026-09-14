use executable_ir::NativeInputType;

pub(super) fn encode(bytes: &mut Vec<u8>, ty: &NativeInputType, field: fn(&mut Vec<u8>, &str, &str)) {
    match ty {
        NativeInputType::F32Array => field(bytes, "input-type", "internal-f32-array-ref"),
        NativeInputType::Int => field(bytes, "input-type", "int"),
        NativeInputType::Bool => field(bytes, "input-type", "bool"),
        NativeInputType::String => field(bytes, "input-type", "string"),
        NativeInputType::StringList => field(bytes, "input-type", "internal-string-list-ref"),
        NativeInputType::Struct(id) => {
            field(bytes, "input-type", "internal-struct-ref");
            field(bytes, "schema", &id.to_string());
        }
        NativeInputType::Email => field(bytes, "input-type", "email"),
        NativeInputType::Url => field(bytes, "input-type", "url"),
        NativeInputType::Slug => field(bytes, "input-type", "slug"),
        NativeInputType::DomainInt { domain, ranges } => {
            field(bytes, "input-type", "domain-int");
            field(bytes, "domain", &domain.to_string());
            for (min, max) in ranges { field(bytes, "range", &format!("{min}:{max}")); }
        }
        NativeInputType::DomainBool { domain } => {
            field(bytes, "input-type", "domain-bool");
            field(bytes, "domain", &domain.to_string());
        }
        NativeInputType::Upload => field(bytes, "input-type", "Upload"),
        NativeInputType::Image => field(bytes, "input-type", "Image"),
        NativeInputType::DomainString { domain, lengths } => {
            field(bytes, "input-type", "domain-string");
            field(bytes, "domain", &domain.to_string());
            for (min, max) in lengths { field(bytes, "length", &format!("{min}:{max}")); }
        }
    }
}
