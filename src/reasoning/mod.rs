pub mod holdings_extractor;
pub mod analyzer;
pub mod argument_graph;

pub use holdings_extractor::{HoldingNode, ExtractedHolding, HoldingConfidence, extract_holdings_from_text};
pub use analyzer::{AnalysisResult, fetch_opinion_and_analyze};
pub use argument_graph::{ArgumentGraph, CaseNode, Edge, EdgeType, ArgumentClaim};
