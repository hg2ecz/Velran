use language_core::{Program, ValueType};

pub(super) fn representation(program: &Program, ty: ValueType) -> ValueType {
    program.representation_type(ty).unwrap_or(ty)
}

pub(super) fn represented_as(program: &Program, actual: ValueType, expected: ValueType) -> bool {
    representation(program, actual) == expected
}

pub(super) fn display(program: &Program, ty: ValueType) -> String {
    match ty {
        ValueType::Credential(purpose) => purpose.source_name().to_string(),
        ValueType::Domain(id) => program
            .domain_type_by_id(id)
            .map(|domain| {
                domain
                    .name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&domain.name)
                    .to_string()
            })
            .unwrap_or_else(|| format!("<domain:{id}>")),
        other => format!("{other:?}"),
    }
}
