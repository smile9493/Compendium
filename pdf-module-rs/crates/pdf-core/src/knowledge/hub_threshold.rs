//! Hub in-degree threshold for load-bearing and cognitive diversity warnings.

use std::path::Path;

use crate::error::PdfResult;
use crate::knowledge::cognitive_diversity::DEFAULT_HUB_IN_DEGREE;
use crate::knowledge::index::graph::GraphIndex;
use crate::management::config_manager::ConfigManager;

/// Config key override (`set_config hub_in_degree=N`).
pub const KEY_HUB_IN_DEGREE: &str = "hub_in_degree";

/// Resolve hub threshold: config override, else adaptive from graph.
pub fn hub_threshold_for_kb(knowledge_base: &Path, graph: &GraphIndex) -> PdfResult<usize> {
    let mut cm = ConfigManager::new(knowledge_base);
    cm.load()?;
    if let Some(v) = cm.get(KEY_HUB_IN_DEGREE)
        && let Ok(n) = v.trim().parse::<usize>()
    {
        return Ok(n.clamp(3, 20));
    }
    Ok(graph.compute_hub_threshold(DEFAULT_HUB_IN_DEGREE))
}
