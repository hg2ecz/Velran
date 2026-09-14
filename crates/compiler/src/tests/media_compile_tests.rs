use super::*;

#[test]
fn accepts_typed_image_upload_and_renderer() {
    let src = r#"
#[action] fn save(ctx: ActionContext, hero: Image) -> Result<Json, PageError> { return Ok(json(hero)); }
route save POST "/save" upload hero<Image> to "media" publish auth user => save;
#[page] fn show(ctx: PageContext, hero: Image) -> Result<Html, PageError> { return Ok(html {<figure>@image(hero, "Hero image")</figure>}); }
route show GET "/show" query hero<Image> public => show;
"#;
    let p = compile_source(src).unwrap();
    assert!(
        p.routes
            .iter()
            .find(|r| r.name == "save")
            .unwrap()
            .upload
            .as_ref()
            .unwrap()
            .image
    );
}
#[test]
fn image_metadata_fields_compile() {
    let src = r#"
#[action] fn save(ctx: ActionContext, hero: Image) -> Result<Json, PageError> { return Ok(json(hero.bytes)); }
route save POST "/save" upload hero<Image> to "media" publish auth user => save;
#[page] fn show(ctx: PageContext, hero: Image) -> Result<Html, PageError> { return Ok(html {<p>{{ hero.width }}x{{ hero.height }}</p>}); }
route show GET "/show" query hero<Image> public => show;
"#;
    compile_source(src).expect("Image metadata fields should compile");
}

#[test]
fn rejects_image_direct_interpolation_and_attribute_context() {
    let direct = r#"
#[page] fn show(ctx: PageContext, hero: Image) -> Result<Html, PageError> { return Ok(html {<div>{{ hero }}</div>}); }
route show GET "/" query hero<Image> public => show;
"#;
    assert!(compile_source(direct).is_err());
    let attr = r#"
#[page] fn show(ctx: PageContext, hero: Image) -> Result<Html, PageError> { return Ok(html {<div class="@image(hero, \"x\")">x</div>}); }
route show GET "/" query hero<Image> public => show;
"#;
    assert!(compile_source(attr).is_err());
}
#[test]
fn image_destination_must_be_url_safe() {
    let src = r#"
#[action] fn save(ctx: ActionContext, hero: Image) -> Result<Json, PageError> { return Ok(json(hero)); }
route save POST "/save" upload hero<Image> to "media files" publish auth user => save;
"#;
    assert!(compile_source(src).is_err());
}
#[test]
fn image_upload_requires_explicit_publish_transition() {
    let src = r#"
#[action] fn save(ctx: ActionContext, hero: Image) -> Result<Json, PageError> { return Ok(json(hero)); }
route save POST "/save" upload hero<Image> to "media" auth user => save;
"#;
    let err = compile_source(src).expect_err("Image must not become public implicitly");
    assert!(err.to_string().contains("SEC-FILE-001"));
}

#[test]
fn raw_upload_cannot_be_published() {
    let src = r#"
#[action] fn save(ctx: ActionContext, file: Upload) -> Result<Json, PageError> { return Ok(json(true)); }
route save POST "/save" upload file<Upload> to "private" publish auth user => save;
"#;
    let err = compile_source(src).expect_err("raw Upload must remain private");
    assert!(err.to_string().contains("SEC-FILE-002"));
}
