#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PublicError {
    BadRequest,
    NotFound,
    Forbidden,
    Conflict,
}

impl PublicError {
    pub const ALL: [Self; 4] = [
        Self::BadRequest,
        Self::NotFound,
        Self::Forbidden,
        Self::Conflict,
    ];

    pub fn source_name(self) -> &'static str {
        match self {
            Self::BadRequest => "badRequest",
            Self::NotFound => "notFound",
            Self::Forbidden => "forbidden",
            Self::Conflict => "conflict",
        }
    }

    pub fn from_source_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|value| value.source_name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::PublicError;
    use crate::AppError;

    #[test]
    fn public_error_mapping_never_produces_internal_or_database_errors() {
        for error in PublicError::ALL {
            assert!(matches!(
                AppError::from(error),
                AppError::BadRequest
                    | AppError::NotFound
                    | AppError::Forbidden
                    | AppError::Conflict
            ));
        }
    }
}
