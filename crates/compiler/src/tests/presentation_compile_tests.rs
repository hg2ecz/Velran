use super::*;

mod m29_component_layout_compile_tests {
    use super::*;

    #[test]
    fn compiles_typed_component_and_layout() {
        let src = r#"
model Article {
    id: i64
    title: String
}
#[component] fn ArticleCard(article: Article) -> Html {
    html {<article><h2>{{ article.title }}</h2></article>}
}
#[layout] fn Main(title: String) -> Html {
    html {<html><head><title>{{ title }}</title></head><body><main>@content</main></body></html>}
}
#[query] fn loadArticle(db: Db, id: i64) -> Result<Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
#[page] fn article(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let article = loadArticle(db, id)?;
    return Ok(html {
        @layout(Main, "Article") {
            @component(ArticleCard, article)
        }
    });
}
route article GET "/articles/:id<i64>" public => article;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.components.len(), 1);
        assert_eq!(p.layouts.len(), 1);
    }

    #[test]
    fn component_accepts_validated_request_string() {
        let src = r#"
#[component] fn Badge(text: String) -> Html { html {<strong>{{ text }}</strong>} }
#[page] fn home(ctx: PageContext, name: String) -> Result<Html, PageError> {
    return Ok(html {@component(Badge, name)});
}
route home GET "/" query name<String> public => home;
"#;
        compile_source(src)
            .expect("validated request strings should be valid presentation arguments");
    }

    #[test]
    fn component_cannot_launder_secret_scalar() {
        let src = r#"
model Credential {
    id: i64
    token: Secret<String>
}
#[component] fn Badge(text: String) -> Html { html {<strong>{{ text }}</strong>} }
#[query] fn load(db: Db, id: i64) -> Result<Credential, DbError> sql {
    SELECT id, token FROM credentials WHERE id = :id
}
#[page] fn home(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let credential = load(db, id)?;
    return Ok(html {@component(Badge, credential.token)});
}
route home GET "/" query id<i64> public => home;
"#;
        let err = compile_source(src).expect_err("template calls must not declassify secrets");
        assert!(err.to_string().contains("SEC-DATA-002"));
    }

    #[test]
    fn rejects_layout_without_exactly_one_content_slot() {
        let src = r#"
#[layout] fn Bad(title: String) -> Html { html {<main>{{ title }}</main>} }
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {x}); }
route home GET "/" public => home;
"#;
        assert!(compile_source(src).is_err());
    }

    #[test]
    fn rejects_component_inside_attribute_context() {
        let src = r#"
#[component] fn Badge(text: String) -> Html { html {<b>{{ text }}</b>} }
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {<div class="@component(Badge, "x")">x</div>});
}
route home GET "/" public => home;
"#;
        assert!(matches!(
            compile_source(src),
            Err(CompileError::UnsafeHtml(_))
        ));
    }

    #[test]
    fn rejects_template_cycles() {
        let src = r#"
#[component] fn A(x: String) -> Html { html {@component(B, x)} }
#[component] fn B(x: String) -> Html { html {@component(A, x)} }
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {x}); }
route home GET "/" public => home;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m32_markdown_compiler_tests {
    use super::*;

    #[test]
    fn accepts_markdown_string_in_content_position() {
        let src = r#"
#[page] fn article(ctx: PageContext, body: String) -> Result<Html, PageError> {
    return Ok(html {<article>@markdown(body)</article>});
}
route article GET "/" query body<String> public => article;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.routes.len(), 1);
    }

    #[test]
    fn rejects_markdown_non_string() {
        let src = r#"
#[page] fn article(ctx: PageContext, id: i64) -> Result<Html, PageError> {
    return Ok(html {<article>@markdown(id)</article>});
}
route article GET "/" query id<i64> public => article;
"#;
        assert!(compile_source(src).is_err());
    }

    #[test]
    fn rejects_markdown_inside_attribute() {
        let src = r#"
#[page] fn article(ctx: PageContext, body: String) -> Result<Html, PageError> {
    return Ok(html {<div class="@markdown(body)">x</div>});
}
route article GET "/" query body<String> public => article;
"#;
        assert!(matches!(
            compile_source(src),
            Err(CompileError::UnsafeHtml(_))
        ));
    }
}
