use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use crate::pager::Pager;
use crate::b_minus_node::BNode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BMinusMetaPage {
    pub root_id: u32,
}

pub struct BMinusTree {
    pub pager: Arc<Mutex<Pager>>,
    pub root_id: u32,
}

impl BMinusTree {
    pub fn new(pager: Arc<Mutex<Pager>>) -> Self {
        let mut p = pager.lock().unwrap();
        
        // Let's reserve page 1 for the BMinusTree meta page (Page 0 is B+ Tree meta page)
        // Actually, this is sharing a single Pager (file db). B+ tree and B- tree in the same file!
        // To not conflict, let's use a separate file for B- tree. The caller will pass a dedicated pager.
        if p.num_pages == 1 {
            let root_id = p.allocate_page();
            let root_node = BNode::new(root_id);
            p.write_page(root_id, &root_node.serialize()).unwrap();
            
            let meta = BMinusMetaPage { root_id };
            let meta_bytes = bincode::serialize(&meta).unwrap();
            p.write_page(0, &meta_bytes).unwrap();
            
            drop(p);
            return Self { pager, root_id };
        }
        
        let meta_bytes = p.read_page(0).unwrap();
        let meta: BMinusMetaPage = bincode::deserialize(&meta_bytes).unwrap();
        let root_id = meta.root_id;
        drop(p);
        
        Self { pager, root_id }
    }

    pub fn get_node(&self, id: u32) -> BNode {
        let mut p = self.pager.lock().unwrap();
        let bytes = p.read_page(id).unwrap();
        BNode::deserialize(&bytes).unwrap()
    }

    pub fn save_node(&self, node: &BNode) {
        let mut p = self.pager.lock().unwrap();
        p.write_page(node.id, &node.serialize()).unwrap();
    }

    fn save_meta(&mut self) {
        let meta = BMinusMetaPage { root_id: self.root_id };
        let meta_bytes = bincode::serialize(&meta).unwrap();
        let mut p = self.pager.lock().unwrap();
        p.write_page(0, &meta_bytes).unwrap();
    }

    pub fn insert(&mut self, key: u64, value: String) {
        let root = self.get_node(self.root_id);
        self.insert_into_node(root, key, value);
    }

    fn insert_into_node(&mut self, mut node: BNode, key: u64, value: String) {
        if node.is_leaf() {
            match node.keys.binary_search(&key) {
                Ok(pos) => {
                    node.values[pos] = value;
                    self.save_node(&node);
                    return;
                }
                Err(pos) => {
                    node.keys.insert(pos, key);
                    node.values.insert(pos, value);
                    self.save_node(&node);

                    if node.is_overflowing() {
                        self.split_node(node);
                    }
                }
            }
        } else {
            let pos = match node.keys.binary_search(&key) {
                Ok(pos) => {
                    node.values[pos] = value;
                    self.save_node(&node);
                    return;
                }
                Err(pos) => pos,
            };
            let child_id = node.children[pos];
            let child = self.get_node(child_id);
            self.insert_into_node(child, key, value);
        }
    }

    fn split_node(&mut self, mut node: BNode) {
        let split_at = node.keys.len() / 2;
        
        // Pop the middle key/value to promote
        let up_key = node.keys.remove(split_at);
        let up_val = node.values.remove(split_at);
        
        let new_id = self.pager.lock().unwrap().allocate_page();
        
        let mut sibling = BNode {
            id: new_id,
            parent: node.parent,
            keys: node.keys.split_off(split_at),
            values: node.values.split_off(split_at),
            children: if node.is_leaf() {
                Vec::new()
            } else {
                node.children.split_off(split_at + 1)
            },
        };

        // If not leaf, update children's parent pointers
        if !sibling.is_leaf() {
            for &child_id in &sibling.children {
                let mut c = self.get_node(child_id);
                c.parent = Some(sibling.id);
                self.save_node(&c);
            }
        }

        self.save_node(&node);
        self.save_node(&sibling);

        if let Some(parent_id) = node.parent {
            let mut parent = self.get_node(parent_id);
            let pos = parent.keys.binary_search(&up_key).unwrap_or_else(|e| e);
            
            parent.keys.insert(pos, up_key);
            parent.values.insert(pos, up_val);
            parent.children.insert(pos + 1, sibling.id);
            
            self.save_node(&parent);
            if parent.is_overflowing() {
                self.split_node(parent);
            }
        } else {
            // Root split
            let new_root_id = self.pager.lock().unwrap().allocate_page();
            let mut new_root = BNode::new(new_root_id);
            
            new_root.keys.push(up_key);
            new_root.values.push(up_val);
            new_root.children.push(node.id);
            new_root.children.push(sibling.id);
            
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
        
        let mut children_json = Vec::new();
        for &child_id in &node.children {
            children_json.push(self.node_to_json(child_id));
        }

        serde_json::json!({
            "id": id,
            "type": if node.is_leaf() { "leaf" } else { "internal" },
            "keys": node.keys,
            "values": node.values,
            "children": children_json
        })
    }
}
