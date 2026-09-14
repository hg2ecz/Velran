use language_core::{ActionBody, ActionStatement, HttpMethod, PageBody, Program, Route, Statement};

pub(super) fn route_returns_json(program: &Program, route: &Route) -> bool {
    fn page_json(statements: &[Statement]) -> bool {
        match statements.last() {
            Some(Statement::ReturnJson(_))
            | Some(Statement::ReturnJsonProjection(_))
            | Some(Statement::ReturnTypedJson { .. }) => true,
            Some(Statement::Resource { statements, .. }) => page_json(statements),
            Some(Statement::Match { arms, .. }) => {
                arms.iter().all(|arm| page_json(&arm.statements))
            }
            _ => false,
        }
    }
    fn action_json(statements: &[ActionStatement]) -> bool {
        match statements.last() {
            Some(ActionStatement::ReturnJson(_))
            | Some(ActionStatement::ReturnJsonProjection(_))
            | Some(ActionStatement::ReturnTypedJson { .. }) => true,
            Some(ActionStatement::Resource { statements, .. }) => action_json(statements),
            Some(ActionStatement::Match { arms, .. }) => {
                arms.iter().all(|arm| action_json(&arm.statements))
            }
            _ => false,
        }
    }
    match route.method {
        HttpMethod::Get => program.page(&route.handler).is_some_and(|p| {
            let PageBody::Statements(s) = &p.body;
            page_json(s)
        }),
        HttpMethod::Post => program.action(&route.handler).is_some_and(|a| {
            let ActionBody::Statements(s) = &a.body;
            action_json(s)
        }),
    }
}

pub(super) fn route_returns_typed_json(program: &Program, route: &Route) -> bool {
    fn page_typed(statements: &[Statement]) -> bool {
        match statements.last() {
            Some(Statement::ReturnTypedJson { .. }) => true,
            Some(Statement::Resource { statements, .. }) => page_typed(statements),
            Some(Statement::Match { arms, .. }) => {
                arms.iter().all(|arm| page_typed(&arm.statements))
            }
            _ => false,
        }
    }
    fn action_typed(statements: &[ActionStatement]) -> bool {
        match statements.last() {
            Some(ActionStatement::ReturnTypedJson { .. }) => true,
            Some(ActionStatement::Resource { statements, .. }) => action_typed(statements),
            Some(ActionStatement::Match { arms, .. }) => {
                arms.iter().all(|arm| action_typed(&arm.statements))
            }
            _ => false,
        }
    }
    match route.method {
        HttpMethod::Get => program.page(&route.handler).is_some_and(|p| {
            let PageBody::Statements(s) = &p.body;
            page_typed(s)
        }),
        HttpMethod::Post => program.action(&route.handler).is_some_and(|a| {
            let ActionBody::Statements(s) = &a.body;
            action_typed(s)
        }),
    }
}
