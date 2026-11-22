use std::collections::HashSet;

use crate::metrics::node::NodeConfig;

use super::primitives::{valid_http_url, valid_id, valid_image_url};

pub(crate) fn validate_nodes(nodes: &[NodeConfig]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for node in nodes {
        if node.id.trim().is_empty()
            || node.name.trim().is_empty()
            || node.endpoint.trim().is_empty()
        {
            return Err("node ID, name, and endpoint are required".to_string());
        }
        if !valid_id(&node.id) {
            return Err(format!(
                "node ID '{}' must use letters, numbers, hyphens, or underscores",
                node.id
            ));
        }
        if !ids.insert(node.id.as_str()) {
            return Err(format!("duplicate node id '{}'", node.id));
        }
        if !(1..=86_400).contains(&node.interval_secs) {
            return Err(format!(
                "node '{}' scrape interval must be between 1 and 86400 seconds",
                node.id
            ));
        }
        if let Some(icon) = node.icon.as_deref().filter(|icon| !icon.trim().is_empty())
            && !valid_image_url(icon)
        {
            return Err(format!(
                "server '{}' icon must be an http(s) URL or an uploaded icon path",
                node.id
            ));
        }
        if !node.is_local() && !valid_http_url(&node.endpoint) {
            return Err(format!(
                "node '{}' endpoint must be `local` or a valid http(s) URL",
                node.id
            ));
        }
    }
    Ok(())
}
