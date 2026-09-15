#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinExecutionKind {
    Simple,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinMetadata {
    pub source_name: &'static str,
    pub min_args: usize,
    pub max_args: usize,
    pub instruction_cost: u64,
    pub uses_request_state: bool,
    pub execution_kind: BuiltinExecutionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinFunction {
    Sin,
    Cos,
    Sqrt,
    Abs,
    Ln,
    Log10,
    Log,
    Exp,
    Pow,
    Round,
    Floor,
    Ceil,
    MonotonicNanos,
    ToF32,
    StringLen,
    Trim,
    TrimStart,
    TrimEnd,
    Lower,
    Upper,
    Contains,
    StartsWith,
    EndsWith,
    Replace,
    Split,
    SplitBounded,
    Substring,
    IndexOf,
    LastIndexOf,
    CharAt,
    Repeat,
    DictNew,
    ContainsKey,
    RemoveKey,
    RegexMatch,
    RegexReplace,
    RegexCaptures,
    PasswordHash,
    PasswordVerify,
    NewSessionToken,
    NewPasswordResetToken,
    NewCsrfToken,
    TokenHash,
    PresentedTokenHash,
    TokenMatches,
    TokenActive,
    SignWebhook,
    VerifyWebhookSignature,
    EncryptUserData,
    DecryptUserData,
    Redact,
    SafeHtmlEmpty,
    SafeHtmlText,
    SafeHtmlElement,
    SafeHtmlLink,
    SafeHtmlConcat,
}

impl BuiltinFunction {
    pub const ALL: [Self; 56] = [
        Self::Sin,
        Self::Cos,
        Self::Sqrt,
        Self::Abs,
        Self::Ln,
        Self::Log10,
        Self::Log,
        Self::Exp,
        Self::Pow,
        Self::Round,
        Self::Floor,
        Self::Ceil,
        Self::MonotonicNanos,
        Self::ToF32,
        Self::StringLen,
        Self::Trim,
        Self::TrimStart,
        Self::TrimEnd,
        Self::Lower,
        Self::Upper,
        Self::Contains,
        Self::StartsWith,
        Self::EndsWith,
        Self::Replace,
        Self::Split,
        Self::SplitBounded,
        Self::Substring,
        Self::IndexOf,
        Self::LastIndexOf,
        Self::CharAt,
        Self::Repeat,
        Self::DictNew,
        Self::ContainsKey,
        Self::RemoveKey,
        Self::RegexMatch,
        Self::RegexReplace,
        Self::RegexCaptures,
        Self::PasswordHash,
        Self::PasswordVerify,
        Self::NewSessionToken,
        Self::NewPasswordResetToken,
        Self::NewCsrfToken,
        Self::TokenHash,
        Self::PresentedTokenHash,
        Self::TokenMatches,
        Self::TokenActive,
        Self::SignWebhook,
        Self::VerifyWebhookSignature,
        Self::EncryptUserData,
        Self::DecryptUserData,
        Self::Redact,
        Self::SafeHtmlEmpty,
        Self::SafeHtmlText,
        Self::SafeHtmlElement,
        Self::SafeHtmlLink,
        Self::SafeHtmlConcat,
    ];

    pub fn from_source_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|function| function.metadata().source_name == name)
    }

    pub const fn metadata(self) -> BuiltinMetadata {
        let (source_name, min_args, max_args, instruction_cost, uses_request_state) = match self {
            Self::Sin => ("sin", 1, 1, 15, false),
            Self::Cos => ("cos", 1, 1, 15, false),
            Self::Sqrt => ("sqrt", 1, 1, 15, false),
            Self::Abs => ("abs", 1, 1, 1, false),
            Self::Ln => ("ln", 1, 1, 15, false),
            Self::Log10 => ("log10", 1, 1, 15, false),
            Self::Log => ("log", 2, 2, 20, false),
            Self::Exp => ("exp", 1, 1, 15, false),
            Self::Pow => ("pow", 2, 2, 20, false),
            Self::Round => ("round", 1, 1, 2, false),
            Self::Floor => ("floor", 1, 1, 2, false),
            Self::Ceil => ("ceil", 1, 1, 2, false),
            Self::MonotonicNanos => ("monotonicNanos", 0, 0, 3, true),
            Self::ToF32 => ("toF32", 1, 1, 1, false),
            Self::StringLen => ("stringLen", 1, 1, 1, false),
            Self::Trim => ("trim", 1, 1, 2, false),
            Self::TrimStart => ("trimStart", 1, 1, 2, false),
            Self::TrimEnd => ("trimEnd", 1, 1, 2, false),
            Self::Lower => ("lower", 1, 1, 2, false),
            Self::Upper => ("upper", 1, 1, 2, false),
            Self::Contains => ("contains", 2, 2, 2, false),
            Self::StartsWith => ("startsWith", 2, 2, 2, false),
            Self::EndsWith => ("endsWith", 2, 2, 2, false),
            Self::Replace => ("replace", 3, 3, 4, false),
            Self::Split => ("split", 2, 2, 4, false),
            Self::SplitBounded => ("splitBounded", 3, 3, 4, false),
            Self::Substring => ("substring", 2, 3, 4, false),
            Self::IndexOf => ("indexOf", 2, 2, 3, false),
            Self::LastIndexOf => ("lastIndexOf", 2, 2, 3, false),
            Self::CharAt => ("charAt", 2, 2, 2, false),
            Self::Repeat => ("repeat", 2, 2, 4, false),
            Self::DictNew => ("dict", 0, 0, 1, false),
            Self::ContainsKey => ("containsKey", 2, 2, 2, false),
            Self::RemoveKey => ("removeKey", 2, 2, 3, false),
            Self::RegexMatch => ("regexMatch", 2, 2, 20, false),
            Self::RegexReplace => ("regexReplace", 3, 3, 30, false),
            Self::RegexCaptures => ("regexCaptures", 2, 2, 25, false),
            Self::PasswordHash => ("passwordHash", 1, 1, 80, false),
            Self::PasswordVerify => ("passwordVerify", 2, 2, 80, false),
            Self::NewSessionToken => ("newSessionToken", 0, 0, 8, false),
            Self::NewPasswordResetToken => ("newPasswordResetToken", 0, 0, 8, false),
            Self::NewCsrfToken => ("newCsrfToken", 0, 0, 8, false),
            Self::TokenHash => ("tokenHash", 1, 1, 12, false),
            Self::PresentedTokenHash => ("presentedTokenHash", 1, 1, 12, false),
            Self::TokenMatches => ("tokenMatches", 2, 2, 12, false),
            Self::TokenActive => ("tokenActive", 3, 3, 14, false),
            Self::SignWebhook => ("signWebhook", 2, 2, 18, false),
            Self::VerifyWebhookSignature => ("verifyWebhookSignature", 3, 3, 20, false),
            Self::EncryptUserData => ("encryptUserData", 2, 2, 32, false),
            Self::DecryptUserData => ("decryptUserData", 2, 2, 32, false),
            Self::Redact => ("redact", 1, 1, 2, false),
            Self::SafeHtmlEmpty => ("safeHtmlEmpty", 0, 0, 1, false),
            Self::SafeHtmlText => ("safeHtmlText", 1, 1, 4, false),
            Self::SafeHtmlElement => ("safeHtmlElement", 2, 2, 4, false),
            Self::SafeHtmlLink => ("safeHtmlLink", 2, 2, 5, false),
            Self::SafeHtmlConcat => ("safeHtmlConcat", 2, 2, 3, false),
        };
        let execution_kind = match self {
            Self::RegexMatch | Self::RegexReplace | Self::RegexCaptures => {
                BuiltinExecutionKind::Regex
            }
            _ => BuiltinExecutionKind::Simple,
        };
        BuiltinMetadata {
            source_name,
            min_args,
            max_args,
            instruction_cost,
            uses_request_state,
            execution_kind,
        }
    }

    pub const fn source_name(self) -> &'static str {
        self.metadata().source_name
    }

    pub const fn instruction_cost(self) -> u64 {
        self.metadata().instruction_cost
    }

    pub const fn uses_request_state(self) -> bool {
        self.metadata().uses_request_state
    }

    pub const fn accepts_arity(self, count: usize) -> bool {
        let metadata = self.metadata();
        count >= metadata.min_args && count <= metadata.max_args
    }
}

#[cfg(test)]
mod metadata_tests {
    use super::BuiltinFunction;
    use std::collections::HashSet;

    #[test]
    fn builtin_metadata_has_unique_public_names_and_valid_arity() {
        let mut names = HashSet::new();
        for function in BuiltinFunction::ALL {
            let metadata = function.metadata();
            assert!(names.insert(metadata.source_name));
            assert!(metadata.min_args <= metadata.max_args);
            assert!(metadata.instruction_cost > 0);
            assert_eq!(
                BuiltinFunction::from_source_name(metadata.source_name),
                Some(function)
            );
        }
    }
}
