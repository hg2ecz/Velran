#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModelAccess {
    Unverified,
    Authorized,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ModelType {
    pub name: String,
    pub access: ModelAccess,
}

impl ModelType {
    pub(super) fn unverified(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            access: ModelAccess::Unverified,
        }
    }

    pub(super) fn authorized(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            access: ModelAccess::Authorized,
        }
    }
}
