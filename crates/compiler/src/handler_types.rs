use crate::model_security::ModelType;
use crate::scalar_security::ScalarType;
use language_core::{QueryFunction, QueryReturn, ValueType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HandlerReturnKind {
    Html,
    Redirect,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum StaticType {
    Scalar(ScalarType),
    Model(ModelType),
    OptionalModel(String),
    ListModel(String),
    PureStruct(String),
    PureOption(language_core::PureValueType),
    PureResult {
        ok: language_core::PureValueType,
        err: language_core::PureValueType,
    },
    Upload,
}

impl StaticType {
    pub(super) fn trusted_scalar(value_type: ValueType) -> Self {
        Self::Scalar(ScalarType::trusted(value_type))
    }

    pub(super) fn untrusted_scalar(value_type: ValueType) -> Self {
        Self::Scalar(ScalarType::untrusted(value_type))
    }

    pub(super) fn model(name: impl Into<String>) -> Self {
        Self::Model(ModelType::unverified(name))
    }

    pub(super) fn authorized_model(name: impl Into<String>) -> Self {
        Self::Model(ModelType::authorized(name))
    }

    pub(super) fn scalar(&self) -> Option<ScalarType> {
        match self {
            Self::Scalar(scalar) => Some(scalar.clone()),
            _ => None,
        }
    }

    pub(super) fn is_scalar(&self, expected: ValueType) -> bool {
        self.scalar()
            .map(|scalar| scalar.value_type == expected)
            .unwrap_or(false)
    }
}

pub(super) fn query_static_type(query: &QueryFunction) -> StaticType {
    match &query.return_type {
        QueryReturn::Void | QueryReturn::Changed => StaticType::trusted_scalar(ValueType::Bool),
        QueryReturn::One(model)
            if query.capability == language_core::QueryCapability::Transaction
                && query.mutation_target.is_none() =>
        {
            StaticType::authorized_model(model.clone())
        }
        QueryReturn::One(model) => StaticType::model(model.clone()),
        QueryReturn::Optional(model) => StaticType::OptionalModel(model.clone()),
        QueryReturn::List(model) => StaticType::ListModel(model.clone()),
    }
}
