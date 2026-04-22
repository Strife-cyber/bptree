use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use crate::pager::Pager;
use crate::node::{Node, NodeType, LeafNode, InternalNode, MAX_KEYS};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaPage {
    pub root_id: u32,
}

pub struct BTree {
    pub pager: Arc<Mutex<Pager>>,
    pub root_id: u32,
}

impl BTree {
    pub fn new(pager: Arc<Mutex<Pager>>) -> Self {
        let mut p = pager.lock().unwrap();
        
        // Check if DB is empty / new
        if p.num_pages == 1 {
            // Allocate root node
            let root_id = p.allocate_page();
            let root_node = Node::new_leaf(root_id);
            p.write_page(root_id, &root_node.serialize()).unwrap();
            
            // Write meta page
            let meta = MetaPage { root_id };
            let meta_bytes = bincode::serialize(&meta).unwrap();
            p.write_page(0, &meta_bytes).unwrap();
            
            drop(p);
            return Self { pager, root_id };
        }
        
        let meta_bytes = p.read_page(0).unwrap();
        // Ignore trailing zeros from the page buffer
        let meta: MetaPage = bincode::deserialize(&meta_bytes).unwrap();
        let root_id = meta.root_id;
        drop(p);
        
        Self { pager, root_id }
    }

    pub fn get_node(&self, id: u32) -> Node {
        let mut p = self.pager.lock().unwrap();
        let bytes = p.read_page(id).unwrap();
        Node::deserialize(&bytes).unwrap()
    }

    pub fn save_node(&self, node: &Node) {
        let mut p = self.pager.lock().unwrap();
        p.write_page(node.id, &node.serialize()).unwrap();
    }

    fn save_meta(&mut self) {
        let meta = MetaPage { root_id: self.root_id };
        let meta_bytes = bincode::serialize(&meta).unwrap();
        let mut p = self.pager.lock().unwrap();
        p.write_page(0, &meta_bytes).unwrap();
    }

    pub fn insert(&mut self, key: u64, value: String) {
        let root = self.get_node(self.root_id);
        self.insert_into_node(root, key, value);
    }

    fn insert_into_node(&mut self, mut node: Node, key: u64, value: String) {
        match &mut node.node_type {
            NodeType::Leaf(leaf) => {
                match leaf.keys.binary_search(&key) {
                    Ok(pos) => {
                        // Key exists, update value
                        leaf.values[pos] = value;
                        self.save_node(&node);
                        return;
                    }
                    Err(pos) => {
                        leaf.keys.insert(pos, key);
                        leaf.values.insert(pos, value);
                        self.save_node(&node);

                        if node.is_overflowing() {
                            self.split_node(node);
                        }
                    }
                }
            }
            NodeType::Internal(internal) => {
                let pos = internal.keys.binary_search(&key).unwrap_or_else(|e| e);
                let child_id = internal.children[pos];
                let child = self.get_node(child_id);
                self.insert_into_node(child, key, value);
            }
        }
    }

    fn split_node(&mut self, mut node: Node) {
        let new_id = self.pager.lock().unwrap().allocate_page();
        let mut sibling = match &mut node.node_type {
            NodeType::Leaf(leaf) => {
                let split_at = leaf.keys.len() / 2;
                let mut new_leaf = LeafNode {
                    keys: leaf.keys.split_off(split_at),
                    values: leaf.values.split_off(split_at),
                    next_leaf: leaf.next_leaf,
                };
                leaf.next_leaf = Some(new_id);
                Node {
                    id: new_id,
                    parent: node.parent,
                    node_type: NodeType::Leaf(new_leaf),
                }
            }
            NodeType::Internal(internal) => {
                let split_at = internal.keys.len() / 2;
                // Internal nodes push up the middle key, they don't keep it.
                // But wait, the split algorithm we're doing pushes up `up_key`.
                // Actually, let's defer exact middle-push logic.
                
                let mut new_internal = InternalNode {
                    keys: internal.keys.split_off(split_at + 1),
                    children: internal.children.split_off(split_at + 1),
                };
                
                Node {
                    id: new_id,
                    parent: node.parent,
                    node_type: NodeType::Internal(new_internal),
                }
            }
        };

        // Get the key that goes up
        let up_key = match &mut node.node_type {
            NodeType::Leaf(leaf) => {
                // In a B+ tree, the leaf retains the key, we just copy it up.
                match &sibling.node_type {
                    NodeType::Leaf(l) => l.keys[0],
                    _ => unreachable!(),
                }
            }
            NodeType::Internal(internal) => {
                // In a B+ tree internal node, the key moves up and is removed from the current node.
                internal.keys.pop().unwrap()
            }
        };

        self.save_node(&node);
        self.save_node(&sibling);

        // Update sibling's children's parents if it's an internal node
        if let NodeType::Internal(internal) = &sibling.node_type {
            for &child_id in &internal.children {
                let mut child = self.get_node(child_id);
                child.parent = Some(sibling.id);
                self.save_node(&child);
            }
        }

        if let Some(parent_id) = node.parent {
            let mut parent = self.get_node(parent_id);
            
            if let NodeType::Internal(p_internal) = &mut parent.node_type {
                let pos = p_internal.keys.binary_search(&up_key).unwrap_or_else(|e| e);
                p_internal.keys.insert(pos, up_key);
                p_internal.children.insert(pos + 1, sibling.id);
            }
            self.save_node(&parent);
            
            if parent.is_overflowing() {
                self.split_node(parent);
            }
        } else {
            // Root split
            let new_root_id = self.pager.lock().unwrap().allocate_page();
            let mut new_root = Node::new_internal(new_root_id);
            
            if let NodeType::Internal(r_internal) = &mut new_root.node_type {
                r_internal.keys.push(up_key);
                r_internal.children.push(node.id);
                r_internal.children.push(sibling.id);
            }
            
            self.root_id = new_root_id;
            
            node.parent = Some(new_root_id);
            sibling.parent = Some(new_root_id);
            
            self.save_node(&node);
            self.save_node(&sibling);
            self.save_node(&new_root);
            self.save_meta();
        }
    }

    pub fn get_tree_json(&self) -> serde_json::Value {
        self.node_to_json(self.root_id)
    }

    fn node_to_json(&self, id: u32) -> serde_json::Value {
        let node = self.get_node(id);
        match node.node_type {
            NodeType::Leaf(leaf) => {
                serde_json::json!({
                    "id": id,
                    "type": "leaf",
                    "keys": leaf.keys,
                    "values": leaf.values,
                    "next": leaf.next_leaf
                })
            }
            NodeType::Internal(internal) => {
                let mut children_json = Vec::new();
                for &child_id in &internal.children {
                    children_json.push(self.node_to_json(child_id));
                }
                serde_json::json!({
                    "id": id,
                    "type": "internal",
                    "keys": internal.keys,
                    "children": children_json
                })
            }
        }
    }
}
