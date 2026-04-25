use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// A node in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub label: String,
    pub properties: HashMap<String, String>,
    pub created_at: String,
}

/// An edge (relationship) between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: String,
    pub weight: f32,
    pub created_at: String,
}

/// Knowledge Graph - stores relationships between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: HashMap<String, KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
    pub dir: String,
}

impl KnowledgeGraph {
    pub fn new(dir: &str) -> Self {
        let path = format!("{}/knowledge_graph.json", dir);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(graph) = serde_json::from_str::<KnowledgeGraph>(&content) {
                return graph;
            }
        }

        KnowledgeGraph {
            nodes: HashMap::new(),
            edges: Vec::new(),
            dir: dir.to_string(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, id: &str, label: &str, properties: HashMap<String, String>) {
        use chrono::Utc;
        let now = Utc::now().to_rfc3339();

        self.nodes.insert(id.to_string(), KnowledgeNode {
            id: id.to_string(),
            label: label.to_string(),
            properties,
            created_at: now,
        });
    }

    /// Add an edge between nodes
    pub fn add_edge(&mut self, from: &str, to: &str, relation: &str, weight: f32) {
        use chrono::Utc;
        let now = Utc::now().to_rfc3339();

        let edge_id = format!("{:x}{:x}", 
            from.as_bytes().iter().fold(0u64, |a, &b| a.wrapping_add(b as u64)),
            to.as_bytes().iter().fold(0u64, |a, &b| a.wrapping_add(b as u64))
        );

        self.edges.push(KnowledgeEdge {
            id: edge_id,
            from: from.to_string(),
            to: to.to_string(),
            relation: relation.to_string(),
            weight,
            created_at: now,
        });
    }

    /// Find nodes by label
    pub fn find_nodes(&self, label: &str) -> Vec<&KnowledgeNode> {
        self.nodes.values()
            .filter(|n| n.label.to_lowercase().contains(&label.to_lowercase()))
            .collect()
    }

    /// Get neighbors of a node
    pub fn get_neighbors(&self, node_id: &str) -> Vec<(&KnowledgeNode, &KnowledgeEdge)> {
        let mut neighbors = Vec::new();

        for edge in &self.edges {
            if edge.from == node_id {
                if let Some(node) = self.nodes.get(&edge.to) {
                    neighbors.push((node, edge));
                }
            } else if edge.to == node_id {
                if let Some(node) = self.nodes.get(&edge.from) {
                    neighbors.push((node, edge));
                }
            }
        }

        neighbors
    }

    /// Search the graph for related concepts
    pub fn search(&self, query: &str) -> Vec<&KnowledgeNode> {
        let query_lower = query.to_lowercase();
        self.nodes.values()
            .filter(|n| {
                n.label.to_lowercase().contains(&query_lower) ||
                n.properties.values().any(|v| v.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Get graph statistics
    pub fn stats(&self) -> String {
        let mut relation_counts: HashMap<&str, usize> = HashMap::new();
        for edge in &self.edges {
            *relation_counts.entry(&edge.relation).or_insert(0) += 1;
        }

        let mut output = format!(
            "📊 Knowledge Graph: {} nodes, {} edges\n",
            self.nodes.len(),
            self.edges.len()
        );

        for (relation, count) in &relation_counts {
            output.push_str(&format!("  - {}: {}\n", relation, count));
        }

        output
    }

    /// Save the graph to disk
    pub fn save(&self) {
        let path = format!("{}/knowledge_graph.json", self.dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = KnowledgeGraph::new("/tmp");
        let props = HashMap::new();
        graph.add_node("1", "Rust", props);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.nodes.contains_key("1"));
    }

    #[test]
    fn test_add_edge() {
        let mut graph = KnowledgeGraph::new("/tmp");
        graph.add_node("1", "Rust", HashMap::new());
        graph.add_node("2", "Memory Safety", HashMap::new());
        graph.add_edge("1", "2", "has_property", 0.9);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_find_nodes() {
        let mut graph = KnowledgeGraph::new("/tmp");
        graph.add_node("1", "Rust Programming", HashMap::new());
        graph.add_node("2", "Python", HashMap::new());
        let results = graph.find_nodes("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "Rust Programming");
    }

    #[test]
    fn test_get_neighbors() {
        let mut graph = KnowledgeGraph::new("/tmp");
        graph.add_node("1", "Rust", HashMap::new());
        graph.add_node("2", "Memory Safety", HashMap::new());
        graph.add_node("3", "Concurrency", HashMap::new());
        graph.add_edge("1", "2", "has", 0.9);
        graph.add_edge("1", "3", "has", 0.8);
        let neighbors = graph.get_neighbors("1");
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_search() {
        let mut graph = KnowledgeGraph::new("/tmp");
        let mut props = HashMap::new();
        props.insert("type".to_string(), "language".to_string());
        graph.add_node("1", "Rust", props);
        let results = graph.search("language");
        assert_eq!(results.len(), 1);
    }
}
