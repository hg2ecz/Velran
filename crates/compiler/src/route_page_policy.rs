use language_core::{PageBody, Program, Statement};

pub(super) fn canonical_slug_params<'a>(program: &'a Program, handler: &str) -> Vec<&'a str> {
    fn collect<'a>(statements: &'a [Statement], out: &mut Vec<&'a str>) {
        for statement in statements {
            match statement {
                Statement::CanonicalSlug { param, .. } => out.push(param.as_str()),
                Statement::Resource { statements, .. } => collect(statements, out),
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        collect(&arm.statements, out);
                    }
                }
                _ => {}
            }
        }
    }

    let mut out = Vec::new();
    if let Some(page) = program.page(handler) {
        let PageBody::Statements(statements) = &page.body;
        collect(statements, &mut out);
    }
    out
}

pub(super) fn has_object_auth(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Authorize(_) => true,
        Statement::Resource { statements, .. } => has_object_auth(statements),
        Statement::Match { arms, .. } => arms.iter().any(|arm| has_object_auth(&arm.statements)),
        _ => false,
    })
}
