pub mod local;
pub mod node;
pub mod node_exporter;
pub mod parser;
pub mod service;

use std::collections::HashMap;

use parser::MetricSample;

/// Index samples by metric name; exposed for tests and metric inspection.
#[cfg_attr(not(test), allow(dead_code))]
pub fn index_by_name(samples: &[MetricSample]) -> HashMap<&str, Vec<&MetricSample>> {
    let mut index: HashMap<&str, Vec<&MetricSample>> = HashMap::new();
    for sample in samples {
        index.entry(sample.name.as_str()).or_default().push(sample);
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_samples_by_name() {
        let samples = parser::parse_exposition(
            "node_load1 0.5\nnode_load1 0.7\nnode_uname_info{hostname=\"a\"} 1\n",
        )
        .unwrap();
        let index = index_by_name(&samples);
        assert_eq!(index["node_load1"].len(), 2);
        assert_eq!(index["node_uname_info"].len(), 1);
    }
}
