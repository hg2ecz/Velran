use super::*;

mod tests {
    use super::*;
    const APP: &str = r#"
model Product {
 id: i64
 name: String
 price: i64
}
#[query] fn loadProduct(db: Db, id: i64) -> Result<Product, DbError> sql {
 SELECT id, name, price FROM products WHERE id = :id
}
#[query] fn renameProduct(tx: Transaction, id: i64, name: String) -> Result<Product, DbError> mutates Product by id sql {
 UPDATE products SET name = :name WHERE id = :id RETURNING id, name, price
}
#[page] fn product(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
 let product = loadProduct(db, id)?;
 Ok(html {<h1>{{ product.name }}</h1><p>{{ product.price }}</p>})
}
#[action] fn rename(ctx: ActionContext, db: Db, id: i64, name: String) -> Result<Redirect, PageError> {
 let product = loadProduct(db, id)?;
 authorize product authenticated;
 transaction db {
   renameProduct(tx, product.id, name)?;
 }
 return Ok(redirect(done()));
}
#[page] fn done(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {done}); }
route product GET "/products/:id<i64>" public => product;
route rename POST "/products/:id<i64>/rename" form name<String> auth user => rename;
route done GET "/done" public => done;
"#;
    #[test]
    fn compiles_typed_queries() {
        let p = compile_source(APP).unwrap();
        assert_eq!(p.models.len(), 1);
        assert_eq!(p.queries.len(), 2);
        assert!(p.page("product").unwrap().needs_db);
    }
    #[test]
    fn allows_static_sql_arithmetic_with_bound_values() {
        let src = r#"
model Counter {
    id: i64
    version: i64
}
#[query] fn loadCounter(db: Db, id: i64) -> Result<Counter, DbError> sql {
    SELECT id, version FROM counters WHERE id = :id
}
#[query] fn bump(tx: Transaction, id: i64, version: i64) -> Result<Changed, DbError> mutates Counter by id sql {
    UPDATE counters SET version = version + 1 WHERE id = :id AND version = :version
}
#[action] fn bumpAction(ctx: ActionContext, db: Db, id: i64, version: i64) -> Result<Json, PageError> {
    let counter = loadCounter(db, id)?;
    authorize counter authenticated;
    transaction db {
        bump(tx, counter.id, version)?;
    }
    return Ok(json(true));
}
route bumpRoute POST "/counters/:id<i64>" form version<i64> auth user => bumpAction;
"#;
        assert!(compile_source(src).is_ok());
    }

    #[test]
    fn rejects_sql_interpolation() {
        let bad = APP.replace("WHERE id = :id", "WHERE id = {{ id }}");
        assert!(matches!(
            compile_source(&bad),
            Err(CompileError::UnsafeSql(_))
        ));
    }
    #[test]
    fn rejects_mutation_without_transaction() {
        let bad = APP.replace("renameProduct(tx: Transaction", "renameProduct(db: Db");
        assert!(matches!(
            compile_source(&bad),
            Err(CompileError::UnsafeSql(_))
        ));
    }
}

#[cfg(test)]
mod resource_profile_compile_tests {
    use super::*;

    #[test]
    fn compiles_resource_profile_and_records_source_location() {
        let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    with resource compute {
        let x = 40 + 2;
        return Ok(html {<p>{{ x }}</p>});
    }
}
route home GET "/" public => home;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.resource_uses.len(), 1);
        assert_eq!(p.resource_uses[0].profile, "compute");
        assert_eq!(p.resource_uses[0].source.function, "home");
        assert!(p.resource_uses[0].source.line >= 2);
    }

    #[test]
    fn rejects_nested_resource_profiles() {
        let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    with resource compute {
        with resource heavy {
            return Ok(html {x});
        }
    }
}
route home GET "/" public => home;
"#;
        assert!(matches!(compile_source(src), Err(CompileError::Syntax(_))));
    }
}

#[cfg(test)]
mod m19_json_compile_tests {
    use super::*;

    #[test]
    fn compiles_json_get_and_typed_json_post() {
        let src = r#"
#[page] fn api(ctx: PageContext) -> Result<Json, PageError> {
    let ok = true;
    return Ok(json(ok));
}
#[action] fn echo(ctx: ActionContext, name: String, age: i64, active: bool) -> Result<Json, PageError> {
    return Ok(json(name));
}
route api GET "/api" public => api;
route echo POST "/api/echo" json name<String> age<i64> active<bool> validate name length 1 100 age range 0 150 public => echo;
"#;
        let p = compile_source(src).unwrap();
        let route = p.routes.iter().find(|r| r.name == "echo").unwrap();
        assert_eq!(route.json_fields.len(), 3);
        let page = p.page("api").unwrap();
        let PageBody::Statements(stmts) = &page.body;
        assert!(matches!(stmts.last(), Some(Statement::ReturnJson(_))));
    }

    #[test]
    fn rejects_mixed_form_and_json_body_modes() {
        let src = r#"
#[action] fn echo(ctx: ActionContext, name: String, age: i64) -> Result<Json, PageError> { return Ok(json(name)); }
route echo POST "/api/echo" form name<String> json age<i64> public => echo;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m19_json_return_type_tests {
    use super::*;
    #[test]
    fn json_return_must_match_declared_handler_type() {
        let bad = r#"
#[page] fn api(ctx: PageContext) -> Result<Html, PageError> {
    let ok = true;
    return Ok(json(ok));
}
route api GET "/api" public => api;
"#;
        assert!(compile_source(bad).is_err());
    }
}

#[cfg(test)]
mod statement_boundary_regression_tests {
    use super::*;

    #[test]
    fn semicolon_separates_let_and_return_statements() {
        let src = r#"
#[page] fn api(ctx: PageContext) -> Result<Json, PageError> {
    let ok = true;
    return Ok(json(ok));
}
route api GET "/api" public => api;
"#;
        assert!(compile_source(src).is_ok());
    }
}

#[cfg(test)]
mod m22_rate_route_tests {
    use super::*;

    #[test]
    fn parses_named_rate_policy() {
        let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {ok});
}
route home GET "/" public rate api => home;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.routes[0].rate_policy.as_deref(), Some("api"));
    }

    #[test]
    fn rejects_invalid_rate_policy_name() {
        let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {ok});
}
route home GET "/" public rate bad-name => home;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m26_type_compile_tests {
    use super::*;

    #[test]
    fn compiles_business_types_and_decimal_arithmetic() {
        let src = r#"
model Invoice {
    id: Uuid
    issued: Date
    createdAt: DateTime
    net: Decimal
}
#[action] fn calc(ctx: ActionContext, net: Decimal, tax: Decimal) -> Result<Json, PageError> {
    let gross = net + tax;
    return Ok(json(gross));
}
route calc POST "/api/calc" json net<Decimal> tax<Decimal> public => calc;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.models[0].fields[0].ty, ValueType::Uuid);
        assert_eq!(p.models[0].fields[1].ty, ValueType::Date);
        assert_eq!(p.models[0].fields[2].ty, ValueType::DateTime);
        assert_eq!(p.models[0].fields[3].ty, ValueType::Decimal);
    }

    #[test]
    fn rejects_mixed_int_decimal_arithmetic() {
        let src = r#"
#[action] fn calc(ctx: ActionContext, net: Decimal, count: i64) -> Result<Json, PageError> {
    let bad = net * count;
    return Ok(json(bad));
}
route calc POST "/api/calc" json net<Decimal> count<i64> public => calc;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m27_form_schema_compile_tests {
    use super::*;

    #[test]
    fn reusable_form_schema_expands_into_route_contract() {
        let src = r#"
form ProductForm {
    name<String>
    price<i64>
    validate name length 1 100 price range 0 100000
}
#[action] fn create(ctx: ActionContext, name: String, price: i64) -> Result<Redirect, PageError> {
    return Ok(redirect(done()));
}
#[page] fn done(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {done}); }
route create POST "/products" form ProductForm public => create;
route done GET "/done" public => done;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.forms.len(), 1);
        assert_eq!(p.routes[0].form_schema.as_deref(), Some("ProductForm"));
        assert_eq!(p.routes[0].form_fields.len(), 2);
        assert_eq!(p.routes[0].validations.len(), 2);
    }

    #[test]
    fn form_schema_rejects_bad_validation_type() {
        let src = r#"
form BadForm {
    amount<i64>
    validate amount length 1 10
}
#[action] fn create(ctx: ActionContext, amount: i64) -> Result<Redirect, PageError> { return Ok(redirect("/")); }
route create POST "/" form BadForm public => create;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod public_cache_compile_tests {
    use super::*;
    #[test]
    fn public_cache_requires_user_independent_public_get() {
        let ok = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {<h1>News</h1>}); }
route home GET "/" public cache public ttl 60 => home;
"#;
        let p = compile_source(ok).unwrap();
        assert_eq!(p.routes[0].public_cache.as_ref().unwrap().ttl_secs, 60);
        let bad = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {<p>{{ authPrincipal }}</p>}); }
route home GET "/" public cache public ttl 60 => home;
"#;
        assert!(compile_source(bad).is_err());
    }
    #[test]
    fn invalidation_targets_cached_route() {
        let src = r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {x}); }
#[action] fn publish(ctx: ActionContext) -> Result<Redirect, PageError> { return Ok(redirect(home())); }
route home GET "/" public cache public ttl 60 => home;
route publish POST "/publish" public invalidate cache home => publish;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.routes[1].invalidate_caches, vec!["home"]);
    }
}
