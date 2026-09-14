#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionSourceKind {
    Model,
    OptionalModel,
    ModelList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicProjection {
    pub source: String,
    pub source_kind: ProjectionSourceKind,
    pub model: String,
    pub fields: Vec<String>,
}
