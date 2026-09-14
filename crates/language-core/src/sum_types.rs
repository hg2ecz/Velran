#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PureSumPredicate {
    IsSome,
    IsNone,
    IsOk,
    IsErr,
}
