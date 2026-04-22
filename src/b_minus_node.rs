use serde::{Deserialize, Serialize};

// Standard B-Tree MAX_KEYS (e.g., degree/order). 
// Keeping it small (3 or 4) to visualize splits.
pub const BMINUS_MAX_KEYS: usize = 3;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BNode {
    pub id: u32,
    pub parent: Option<u32>,
    pub keys: Vec<u64>,
    pub values: Vec<String>,
    pub children: Vec<u32>,
}

impl BNode {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            parent: None,
            keys: Vec::new(),
            values: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn is_overflowing(&self) -> bool {
        self.keys.len() > BMINUS_MAX_KEYS
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Failed to serialize b-node")
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
