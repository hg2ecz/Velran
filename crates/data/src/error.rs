use std::fmt;

#[derive(Debug)]
pub enum DataError {
    UnsupportedDatabaseScheme,
    TlsRequired,
    Timeout,
    Sqlx(sqlx::Error),
    #[cfg(feature = "redis")]
    Redis(redis::RedisError),
    RedisUnavailable,
    MissingBind(String),
    UnexpectedBind(String),
    DuplicateBind(String),
    InvalidBindName(String),
    MultipleStatements,
    MalformedSql,
    UnsupportedSqlSyntax,
    InvalidRowShape,
    RowShapeMismatch,
    RowLimitExceeded,
    ResultSizeLimitExceeded,
    InvalidRedisUrl,
    InvalidRedisNamespace,
    InvalidRedisKey,
    RedisValueTooLarge,
    InvalidRedisTtl,
    InvalidRedisResponse,
}

impl DataError {
    pub fn is_unique_violation(&self) -> bool {
        matches!(self, Self::Sqlx(sqlx::Error::Database(db)) if db.is_unique_violation())
    }
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDatabaseScheme => write!(f, "unsupported database URL scheme"),
            Self::TlsRequired => write!(f, "TLS is required by data security policy"),
            Self::Timeout => write!(f, "data operation timed out"),
            Self::Sqlx(_) => write!(f, "database operation failed"),
            #[cfg(feature = "redis")]
            Self::Redis(_) => write!(f, "Redis operation failed"),
            Self::RedisUnavailable => write!(f, "Redis support is not compiled into this build"),
            Self::MissingBind(name) => write!(f, "missing SQL bind :{name}"),
            Self::UnexpectedBind(name) => write!(f, "unexpected SQL bind :{name}"),
            Self::DuplicateBind(name) => write!(f, "duplicate SQL bind :{name}"),
            Self::InvalidBindName(name) => write!(f, "invalid SQL bind name `{name}`"),
            Self::MultipleStatements => write!(
                f,
                "a query declaration may contain exactly one SQL statement"
            ),
            Self::MalformedSql => write!(f, "malformed SQL template"),
            Self::UnsupportedSqlSyntax => {
                write!(f, "SQL syntax is not supported by the safe bind scanner")
            }
            Self::InvalidRowShape => write!(f, "invalid typed row shape"),
            Self::RowShapeMismatch => {
                write!(f, "database row does not match the compiled row shape")
            }
            Self::RowLimitExceeded => write!(f, "database row limit exceeded"),
            Self::ResultSizeLimitExceeded => write!(f, "database result byte limit exceeded"),
            Self::InvalidRedisUrl => write!(f, "invalid Redis URL"),
            Self::InvalidRedisNamespace => write!(f, "invalid Redis namespace"),
            Self::InvalidRedisKey => write!(f, "invalid Redis key"),
            Self::RedisValueTooLarge => write!(f, "Redis value exceeds configured limit"),
            Self::InvalidRedisTtl => write!(f, "Redis TTL is outside configured bounds"),
            Self::InvalidRedisResponse => write!(f, "unexpected Redis response"),
        }
    }
}

impl std::error::Error for DataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlx(error) => Some(error),
            #[cfg(feature = "redis")]
            Self::Redis(error) => Some(error),
            _ => None,
        }
    }
}
