use crate::{SessionStore, TenantId};
use std::time::Duration;

#[test]
fn authenticated_rotation_invalidates_old_session_and_rotates_csrf() {
    let store = SessionStore::new(Duration::from_secs(300), 16);
    let anonymous = store.create().expect("anonymous session");
    let rotated = store
        .rotate_authenticated(
            &anonymous.id,
            "alice".into(),
            false,
            vec!["User".into()],
            vec![TenantId::parse("acme").unwrap()],
            7,
        )
        .expect("authenticated rotation");

    assert_ne!(rotated.id, anonymous.id);
    assert_ne!(rotated.csrf_token, anonymous.csrf_token);
    assert!(rotated.has_membership(&TenantId::parse("acme").unwrap()));
    assert_eq!(store.get(&anonymous.id).expect("old lookup"), None);
    assert_eq!(store.get(&rotated.id).expect("new lookup"), Some(rotated));
}

#[test]
fn logout_style_invalidation_removes_session() {
    let store = SessionStore::new(Duration::from_secs(300), 16);
    let session = store.create().expect("session");
    store.invalidate(&session.id).expect("invalidate");
    assert_eq!(store.get(&session.id).expect("lookup"), None);
}
