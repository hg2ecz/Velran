use super::{F32Value, ImageRef, ValueType};

#[test]
fn value_type_parser_rejects_legacy_spellings() {
    assert_eq!(ValueType::parse("Int"), None);
    assert_eq!(ValueType::parse("Bool"), None);
    assert_eq!(ValueType::parse("F32"), None);
    assert_eq!(ValueType::parse("Array<F32>"), None);
    assert_eq!(ValueType::parse("List<String>"), None);
    assert_eq!(ValueType::parse("Dict<String,String>"), None);
}

#[test]
fn f32_value_has_stable_u32_abi_layout() {
    assert_eq!(std::mem::size_of::<F32Value>(), std::mem::size_of::<u32>());
    assert_eq!(
        std::mem::align_of::<F32Value>(),
        std::mem::align_of::<u32>()
    );
}

#[test]
fn image_ref_round_trips_and_rejects_unsafe_path() {
    let x = ImageRef::new(
        "media/0123456789abcdef0123456789abcdef.upload".into(),
        "image/png".into(),
        640,
        480,
        12345,
    )
    .unwrap();
    assert_eq!(ImageRef::parse(&x.canonical()), Some(x));
    assert!(ImageRef::new("../secret".into(), "image/png".into(), 1, 1, 1).is_none());
    assert!(ImageRef::new("media/.private/x.png".into(), "image/png".into(), 1, 1, 1).is_none());
    assert!(ImageRef::new(".hidden.png".into(), "image/png".into(), 1, 1, 1).is_none());
    assert!(ImageRef::new("media/x".into(), "image/svg+xml".into(), 1, 1, 1).is_none());
}
