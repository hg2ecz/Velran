use language_core::{CredentialPurpose, DataSensitivity, ValueType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TrustLevel {
    Trusted,
    Validated,
    Untrusted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DisclosureEvidence {
    None,
    Authorized,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MutationEvidence {
    pub model: String,
    pub key: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TenantEvidence {
    ActiveRouteTenant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LifecycleEvidence {
    IssuedToken(CredentialPurpose),
    PresentedTokenHash(CredentialPurpose),
    IssuedTokenHash(CredentialPurpose),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScalarType {
    pub value_type: ValueType,
    pub trust: TrustLevel,
    pub sensitivity: DataSensitivity,
    pub disclosure: DisclosureEvidence,
    pub mutation: Option<MutationEvidence>,
    pub tenant: Option<TenantEvidence>,
    pub lifecycle: Option<LifecycleEvidence>,
}

impl ScalarType {
    pub(super) fn trusted(value_type: ValueType) -> Self {
        Self::new(
            value_type,
            TrustLevel::Trusted,
            DataSensitivity::Public,
            DisclosureEvidence::None,
        )
    }

    pub(super) fn validated(value_type: ValueType) -> Self {
        Self::new(
            value_type,
            TrustLevel::Validated,
            DataSensitivity::Public,
            DisclosureEvidence::None,
        )
    }

    pub(super) fn untrusted(value_type: ValueType) -> Self {
        Self::new(
            value_type,
            TrustLevel::Untrusted,
            DataSensitivity::Public,
            DisclosureEvidence::None,
        )
    }

    pub(super) fn presented_token_hash(value_type: ValueType, purpose: CredentialPurpose) -> Self {
        let mut scalar = Self::validated(value_type);
        scalar.lifecycle = Some(LifecycleEvidence::PresentedTokenHash(purpose));
        scalar
    }

    pub(super) fn issued_token(value_type: ValueType, purpose: CredentialPurpose) -> Self {
        let mut scalar = Self::classified(value_type, DataSensitivity::Secret);
        scalar.lifecycle = Some(LifecycleEvidence::IssuedToken(purpose));
        scalar
    }

    pub(super) fn issued_token_hash(value_type: ValueType, purpose: CredentialPurpose) -> Self {
        let mut scalar = Self::classified(value_type, DataSensitivity::Secret);
        scalar.lifecycle = Some(LifecycleEvidence::IssuedTokenHash(purpose));
        scalar
    }

    pub(super) fn validated_refinement(value_type: ValueType, source: &Self) -> Self {
        Self {
            value_type,
            trust: TrustLevel::Validated,
            sensitivity: source.sensitivity,
            disclosure: source.disclosure,
            mutation: None,
            tenant: None,
            lifecycle: None,
        }
    }

    pub(super) fn classified(value_type: ValueType, sensitivity: DataSensitivity) -> Self {
        Self::new(
            value_type,
            TrustLevel::Trusted,
            sensitivity,
            DisclosureEvidence::None,
        )
    }

    pub(super) fn authorized_field(
        value_type: ValueType,
        sensitivity: DataSensitivity,
        model: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        let mut scalar = Self::new(
            value_type,
            TrustLevel::Trusted,
            sensitivity,
            DisclosureEvidence::Authorized,
        );
        scalar.mutation = Some(MutationEvidence {
            model: model.into(),
            key: key.into(),
        });
        scalar
    }

    pub(super) fn active_tenant(value_type: ValueType) -> Self {
        let mut scalar = Self::validated(value_type);
        scalar.tenant = Some(TenantEvidence::ActiveRouteTenant);
        scalar
    }

    pub(super) fn combine(self, other: Self, result_type: ValueType) -> Self {
        Self::new(
            result_type,
            combine_trust(self.trust, other.trust),
            self.sensitivity.max(other.sensitivity),
            combine_disclosure(&self, &other),
        )
    }

    fn new(
        value_type: ValueType,
        trust: TrustLevel,
        sensitivity: DataSensitivity,
        disclosure: DisclosureEvidence,
    ) -> Self {
        Self {
            value_type,
            trust,
            sensitivity,
            disclosure,
            mutation: None,
            tenant: None,
            lifecycle: None,
        }
    }
}

fn combine_trust(left: TrustLevel, right: TrustLevel) -> TrustLevel {
    use TrustLevel::{Trusted, Untrusted, Validated};
    match (left, right) {
        (Untrusted, _) | (_, Untrusted) => Untrusted,
        (Validated, _) | (_, Validated) => Validated,
        (Trusted, Trusted) => Trusted,
    }
}

fn combine_disclosure(left: &ScalarType, right: &ScalarType) -> DisclosureEvidence {
    let sensitive = left.sensitivity.max(right.sensitivity) >= DataSensitivity::Sensitive;
    let all_sensitive_sources_authorized = [left, right]
        .into_iter()
        .filter(|value| value.sensitivity >= DataSensitivity::Sensitive)
        .all(|value| value.disclosure == DisclosureEvidence::Authorized);

    if sensitive && all_sensitive_sources_authorized {
        DisclosureEvidence::Authorized
    } else {
        DisclosureEvidence::None
    }
}
