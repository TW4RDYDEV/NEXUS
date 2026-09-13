//! Typed observation context shared by analyst edits and importer normalization.
#[derive(Debug, Clone, Copy)]
pub enum ObservationSource<'a> {
    Analyst,
    Tool(&'a str),
}
impl<'a> ObservationSource<'a> {
    pub fn as_str(self) -> &'a str {
        match self {
            Self::Analyst => "Analyst",
            Self::Tool(name) => name,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum ObservationConfidence {
    Confirmed,
    High,
    Medium,
}
impl ObservationConfidence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "Confirmed",
            Self::High => "High",
            Self::Medium => "Medium",
        }
    }
}
pub struct NormalizationRecord<'a> {
    pub entity_id: &'a str,
    pub field: &'a str,
    pub value: &'a str,
    pub source: ObservationSource<'a>,
    pub import_id: Option<&'a str>,
    pub confidence: ObservationConfidence,
    pub conflict: bool,
}
