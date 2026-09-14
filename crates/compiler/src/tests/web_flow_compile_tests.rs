use super::*;

mod m43_canonical_url_compile_tests {
    use super::*;

    #[test]
    fn compiles_typed_canonical_slug_invariant() {
        let src = r#"
model Article {
    id: i64
    slug: Slug
    title: String
}
#[query] fn bySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
    SELECT id, slug, title FROM articles WHERE slug = :slug
}
#[page] fn show(ctx: PageContext, db: Db, slug: Slug) -> Result<Html, PageError> {
    let article = bySlug(db, slug)?;
    canonical slug slug from article.slug;
    return Ok(html {<h1>{{ article.title }}</h1>});
}
route article GET "/articles/:slug<Slug>" public => show;
"#;
        let p = compile_source(src).unwrap();
        let page = p.page("show").unwrap();
        let PageBody::Statements(statements) = &page.body;
        assert!(statements.iter().any(|statement| {
            matches!(statement, Statement::CanonicalSlug { param, .. } if param == "slug")
        }));
    }

    #[test]
    fn canonical_slug_must_be_a_slug_path_parameter() {
        let src = r#"
#[page] fn show(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    canonical slug slug from slug;
    return Ok(html {ok});
}
route article GET "/articles" query slug<Slug> public => show;
"#;
        assert!(compile_source(src).is_err());
    }

    #[test]
    fn canonical_slug_is_not_compatible_with_public_page_cache() {
        let src = r#"
#[page] fn show(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    canonical slug slug from slug;
    return Ok(html {ok});
}
route article GET "/articles/:slug<Slug>" public cache public ttl 60 => show;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m44_prg_flash_compile_tests {
    use super::*;

    #[test]
    fn compiles_static_flash_and_flash_directive() {
        let src = r#"
#[page] fn index(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {<main>@flash()<h1>Articles</h1></main>});
}
#[action] fn save(ctx: ActionContext) -> Result<Redirect, PageError> {
    flash success "Article saved";
    return Ok(redirect(articles()));
}
route articles GET "/articles" public => index;
route saveArticle POST "/articles" public => save;
"#;
        let p = compile_source(src).unwrap();
        let action = p.action("save").unwrap();
        let ActionBody::Statements(statements) = &action.body;
        assert!(
            statements
                .iter()
                .any(|s| matches!(s, ActionStatement::Flash(_)))
        );
    }

    #[test]
    fn rejects_dynamic_or_multiple_flash_messages() {
        let dynamic = r#"
#[action] fn save(ctx: ActionContext, message: String) -> Result<Redirect, PageError> {
    flash success message;
    return Ok(redirect("/"));
}
route save POST "/" form message<String> public => save;
"#;
        assert!(compile_source(dynamic).is_err());

        let multiple = r#"
#[action] fn save(ctx: ActionContext) -> Result<Redirect, PageError> {
    flash success "Saved";
    flash info "Done";
    return Ok(redirect("/"));
}
route save POST "/" public => save;
"#;
        assert!(compile_source(multiple).is_err());
    }

    #[test]
    fn flash_page_cannot_use_public_cache() {
        let src = r#"
#[page] fn index(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {@flash()<p>ok</p>});
}
route index GET "/" public cache public ttl 60 => index;
"#;
        assert!(compile_source(src).is_err());
    }
}
