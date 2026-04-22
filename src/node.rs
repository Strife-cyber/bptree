use serde::{Deserialize, Serialize};

// We use a small MAX_KEYS so that the tree splits frequently,
// making it much easier to observe visually in the Web UI.
// In a real database, this would be computed to perfectly fill an OS Page (e.g., 4096 bytes).
// E.g., for 4096 bytes, MAX_KEYS could be ~100-300 depending on value size.
// In a real database, this would be computed to perfectly fill an OS Page (e.g., 4096 bytes).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeType {
    Internal(InternalNode),
    Leaf(LeafNode),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InternalNode {
    pub keys: Vec<u64>,
    pub children: Vec<u32>, // Page numbers of children
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LeafNode {
    pub keys: Vec<u64>,
    pub values: Vec<String>, // We use String here for easy UI rendering
    pub next_leaf: Option<u32>, // Sibling pointer for range scans
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub id: u32,
    pub parent: Option<u32>,
    pub node_type: NodeType,
}

impl Node {
    pub fn new_leaf(id: u32) -> Self {
        Self {
            id,
            parent: None,
            node_type: NodeType::Leaf(LeafNode {
                keys: Vec::new(),
                values: Vec::new(),
                next_leaf: None,
            }),
        }
    }

    pub fn new_internal(id: u32) -> Self {
        Self {
            id,
            parent: None,
            node_type: NodeType::Internal(InternalNode {
                keys: Vec::new(),
                children: Vec::new(),
            }),
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf(_))
    }

    pub fn is_overflowing(&self, max_keys: usize) -> bool {
        match &self.node_type {
            NodeType::Leaf(leaf) => leaf.keys.len() > max_keys,
            NodeType::Internal(internal) => internal.keys.len() > max_keys,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Failed to serialize node")
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
