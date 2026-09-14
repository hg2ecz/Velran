use super::*;

mod m35_object_authorization_compile_tests {
    use super::*;

    fn source(route_auth: &str) -> String {
        format!(
            r#"
model Article {{
    id: i64
    authorUsername: String
    title: String
}}
#[query] fn loadArticle(db: Db, id: i64) -> Result<Article, DbError> sql {{
    SELECT id, authorUsername, title FROM articles WHERE id = :id
}}
#[query] fn updateTitle(tx: Transaction, id: i64, title: String) -> Result<(), DbError> mutates Article by id sql {{
    UPDATE articles SET title = :title WHERE id = :id
}}
#[action] fn edit(ctx: ActionContext, db: Db, id: i64, title: String) -> Result<Redirect, PageError> {{
    let article = loadArticle(db, id)?;
    authorize article owner authorUsername or role Publisher or role Admin;
    transaction db {{
        updateTitle(tx, article.id, title)?;
    }}
    return Ok(redirect(done()));
}}
#[page] fn done(ctx: PageContext) -> Result<Html, PageError> {{ return Ok(html {{done}}); }}
route done GET "/done" public => done;
route edit POST "/articles/:id<i64>" form title<String> {route_auth} => edit;
"#
        )
    }

    #[test]
    fn compiles_owner_or_role_guard_and_action_read_query() {
        let p = compile_source(&source("auth user")).unwrap();
        let action = p.action("edit").unwrap();
        let ActionBody::Statements(stmts) = &action.body;
        assert!(
            stmts
                .iter()
                .any(|s| matches!(s, ActionStatement::LetQuery { .. }))
        );
        let rule = stmts
            .iter()
            .find_map(|s| match s {
                ActionStatement::Authorize(r) => Some(r),
                _ => None,
            })
            .unwrap();
        assert_eq!(rule.object, "article");
        assert_eq!(
            &rule.mode,
            &AuthorizationMode::Owner {
                field: "authorUsername".into()
            }
        );
        assert_eq!(rule.allow_roles, vec!["Publisher", "Admin"]);
    }

    #[test]
    fn rejects_object_authorization_on_public_route() {
        assert!(compile_source(&source("")).is_err());
    }

    #[test]
    fn rejects_non_string_owner_field() {
        let src = r#"
model Article {
    id: i64
    ownerId: i64
}
#[query] fn loadArticle(db: Db, id: i64) -> Result<Article, DbError> sql {
    SELECT id, ownerId FROM articles WHERE id = :id
}
#[page] fn show(ctx: PageContext, db: Db, id: i64) -> Result<Json, PageError> {
    let article = loadArticle(db, id)?;
    authorize article owner ownerId;
    return Ok(json(article));
}
route show GET "/articles/:id<i64>" auth user => show;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m37_domain_object_compile_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_app() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("velran-m37-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compiles_domain_object_and_namespaced_route_handler() {
        let src = r#"
object Article {
    model {
        id: i64
        slug: Slug
        title: String
        authorUsername: String
    }

    #[query] fn bySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
        SELECT id, slug, title, authorUsername FROM articles WHERE slug = :slug
    }

    #[page] fn show(ctx: PageContext, db: Db, slug: Slug) -> Result<Html, PageError> {
        let article = Article.bySlug(db, slug)?;
        return Ok(html {<h1>{{ article.title }}</h1>});
    }
}
route articleShow GET "/cikk/:slug<Slug>" public => Article.show;
"#;
        let p = compile_source(src).unwrap();
        assert!(p.model("Article").is_some());
        assert!(p.query("Article__bySlug").is_some());
        assert!(p.page("Article__show").is_some());
        assert_eq!(p.routes[0].handler, "Article__show");
    }

    #[test]
    fn domain_object_members_work_across_modules() {
        let dir = temp_app();
        fs::write(dir.join("main.vrn"), "mod article;\nmod routes;\n").unwrap();
        fs::write(
            dir.join("article.vrn"),
            r#"
object Article {
    model {
        id: i64
        slug: Slug
        title: String
    }
    #[query] fn bySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
        SELECT id, slug, title FROM articles WHERE slug = :slug
    }
    #[page] fn show(ctx: PageContext, db: Db, slug: Slug) -> Result<Json, PageError> {
        let article = Article.bySlug(db, slug)?;
        return Ok(json(expose(article, id, slug, title)));
    }
}
"#,
        )
        .unwrap();
        fs::write(
            dir.join("routes.vrn"),
            r#"
route articleShow GET "/cikk/:slug<Slug>" public => article::Article.show;
"#,
        )
        .unwrap();
        let p = compile_file(dir.join("main.vrn")).unwrap();
        assert!(p.page("article::Article__show").is_some());
        assert_eq!(p.routes[0].handler, "article::Article__show");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn same_domain_object_name_isolated_by_module_namespace() {
        let dir = temp_app();
        fs::write(
            dir.join("main.vrn"),
            r#"mod a;
mod b;
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
route home GET "/" public => home;
"#,
        )
        .unwrap();
        let obj = r#"
object Article {
    model {
        id: i64
    }
}
"#;
        fs::write(dir.join("a.vrn"), obj).unwrap();
        fs::write(dir.join("b.vrn"), obj).unwrap();
        let p = compile_file(dir.join("main.vrn")).unwrap();
        assert!(p.model("a::Article").is_some());
        assert!(p.model("b::Article").is_some());
        assert!(p.model("Article").is_none());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_nested_domain_objects() {
        let src = r#"
object Article {
    model {
        id: i64
    }
    object Hidden {
        model {
            id: i64
        }
    }
}
route x GET "/" public => missing;
"#;
        assert!(compile_source(src).is_err());
    }
}
