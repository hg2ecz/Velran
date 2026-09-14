use language_core::{InlineHint, PureParamType, PureReturnType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPureFunction {
    pub(crate) name: String,
    pub(crate) inline: InlineHint,
    pub(crate) params: Vec<(String, PureParamType)>,
    pub(crate) return_type: PureReturnType,
    pub(crate) body: crate::VerifiedScalarBody,
}

impl VerifiedPureFunction {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn inline(&self) -> InlineHint {
        self.inline
    }

    pub fn params(&self) -> &[(String, PureParamType)] {
        &self.params
    }

    pub fn return_type(&self) -> PureReturnType {
        self.return_type.clone()
    }

    pub fn body(&self) -> &crate::VerifiedScalarBody {
        &self.body
    }

    pub fn numeric_body(&self) -> Option<&crate::VerifiedNumericBody> {
        self.body.numeric_body()
    }
}
