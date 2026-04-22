use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use crate::pager::Pager;
use crate::b_minus_node::BNode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BMinusMetaPage {
    pub root_id: u32,
    pub max_keys: usize,
}

pub struct BMinusTree {
    pub pager: Arc<Mutex<Pager>>,
    pub root_id: u32,
    pub max_keys: usize,
}

impl BMinusTree {
    pub fn new(pager: Arc<Mutex<Pager>>, max_keys: usize) -> Self {
        let mut p = pager.lock().unwrap();
        
        if p.num_pages == 1 {
            let root_id = p.allocate_page();
            let root_node = BNode::new(root_id);
            p.write_page(root_id, &root_node.serialize()).unwrap();
            
            let meta = BMinusMetaPage { root_id, max_keys };
            let meta_bytes = bincode::serialize(&meta).unwrap();
            p.write_page(0, &meta_bytes).unwrap();
            
            drop(p);
            return Self { pager, root_id, max_keys };
        }
        
        let meta_bytes = p.read_page(0).unwrap();
        let meta: BMinusMetaPage = bincode::deserialize(&meta_bytes).unwrap();
        let root_id = meta.root_id;
        let db_max_keys = meta.max_keys;
        drop(p);
        
        Self { pager, root_id, max_keys: db_max_keys }
    }

    pub fn reset(&mut self, max_keys: usize) {
        let mut p = self.pager.lock().unwrap();
        p.reset().unwrap();
        self.max_keys = max_keys;
        
        let root_id = p.allocate_page();
        let root_node = BNode::new(root_id);
        p.write_page(root_id, &root_node.serialize()).unwrap();
        
        let meta = BMinusMetaPage { root_id, max_keys };
        let meta_bytes = bincode::serialize(&meta).unwrap();
        p.write_page(0, &meta_bytes).unwrap();
        self.root_id = root_id;
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
        let meta = BMinusMetaPage { root_id: self.root_id, max_keys: self.max_keys };
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

                    if node.is_overflowing(self.max_keys) {
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

    pub fn delete(&mut self, key: u64) {
        let root = self.get_node(self.root_id);
        self.delete_from_node(root, key);
        
        let mut root = self.get_node(self.root_id);
        if root.keys.is_empty() && !root.is_leaf() {
            self.root_id = root.children[0];
            let mut new_root = self.get_node(self.root_id);
            new_root.parent = None;
            self.save_node(&new_root);
            self.save_meta();
        }
    }

    fn delete_from_node(&mut self, mut node: BNode, key: u64) {
        match node.keys.binary_search(&key) {
            Ok(pos) => {
                if node.is_leaf() {
                    node.keys.remove(pos);
                    node.values.remove(pos);
                    self.save_node(&node);
                } else {
                    let mut pred = self.get_node(node.children[pos]);
                    while !pred.is_leaf() {
                        pred = self.get_node(*pred.children.last().unwrap());
                    }
                    
                    let pred_key = *pred.keys.last().unwrap();
                    let pred_val = pred.values.last().unwrap().clone();
                    
                    node.keys[pos] = pred_key;
                    node.values[pos] = pred_val;
                    self.save_node(&node);
                    
                    let left_child = self.get_node(node.children[pos]);
                    self.delete_from_node(left_child, pred_key);
                }
            }
            Err(pos) => {
                if !node.is_leaf() {
                    let child = self.get_node(node.children[pos]);
                    self.delete_from_node(child, key);
                }
            }
        }
        
        let node = self.get_node(node.id);
        if let Some(parent_id) = node.parent {
            let min_keys = self.max_keys / 2;
            if node.keys.len() < min_keys {
                self.rebalance(parent_id, node.id);
            }
        }
    }
    
    fn rebalance(&mut self, parent_id: u32, child_id: u32) {
        let mut parent = self.get_node(parent_id);
        let pos = parent.children.iter().position(|&id| id == child_id).unwrap();
        
        let min_keys = self.max_keys / 2;
        
        // Try borrow left
        if pos > 0 {
            let mut left_sibling = self.get_node(parent.children[pos - 1]);
            if left_sibling.keys.len() > min_keys {
                let mut child = self.get_node(child_id);
                
                let borrow_k = left_sibling.keys.pop().unwrap();
                let borrow_v = left_sibling.values.pop().unwrap();
                let parent_k = parent.keys[pos - 1];
                let parent_v = parent.values[pos - 1].clone();
                
                parent.keys[pos - 1] = borrow_k;
                parent.values[pos - 1] = borrow_v;
                
                child.keys.insert(0, parent_k);
                child.values.insert(0, parent_v);
                
                if !left_sibling.is_leaf() {
                    let borrow_c = left_sibling.children.pop().unwrap();
                    let mut bc_node = self.get_node(borrow_c);
                    bc_node.parent = Some(child.id);
                    self.save_node(&bc_node);
                    child.children.insert(0, borrow_c);
                }
                
                self.save_node(&left_sibling);
                self.save_node(&child);
                self.save_node(&parent);
                return;
            }
        }
        
        // Try borrow right
        if pos < parent.children.len() - 1 {
            let mut right_sibling = self.get_node(parent.children[pos + 1]);
            if right_sibling.keys.len() > min_keys {
                let mut child = self.get_node(child_id);
                
                let borrow_k = right_sibling.keys.remove(0);
                let borrow_v = right_sibling.values.remove(0);
                let parent_k = parent.keys[pos];
                let parent_v = parent.values[pos].clone();
                
                parent.keys[pos] = borrow_k;
                parent.values[pos] = borrow_v;
                
                child.keys.push(parent_k);
                child.values.push(parent_v);
                
                if !right_sibling.is_leaf() {
                    let borrow_c = right_sibling.children.remove(0);
                    let mut bc_node = self.get_node(borrow_c);
                    bc_node.parent = Some(child.id);
                    self.save_node(&bc_node);
                    child.children.push(borrow_c);
                }
                
                self.save_node(&right_sibling);
                self.save_node(&child);
                self.save_node(&parent);
                return;
            }
        }
        
        // Merge
        if pos > 0 {
            let mut left_sibling = self.get_node(parent.children[pos - 1]);
            let mut child = self.get_node(child_id);
            
            let parent_k = parent.keys.remove(pos - 1);
            let parent_v = parent.values.remove(pos - 1);
            parent.children.remove(pos);
            
            left_sibling.keys.push(parent_k);
            left_sibling.values.push(parent_v);
            
            left_sibling.keys.append(&mut child.keys);
            left_sibling.values.append(&mut child.values);
            
            if !child.is_leaf() {
                for &c_id in &child.children {
                    let mut c_node = self.get_node(c_id);
                    c_node.parent = Some(left_sibling.id);
                    self.save_node(&c_node);
                }
                left_sibling.children.append(&mut child.children);
            }
            
            self.save_node(&left_sibling);
            self.save_node(&parent);
            
            if let Some(gp_id) = parent.parent {
                if parent.keys.len() < min_keys {
                    self.rebalance(gp_id, parent.id);
                }
            }
        } else {
            let mut right_sibling = self.get_node(parent.children[pos + 1]);
            let mut child = self.get_node(child_id);
            
            let parent_k = parent.keys.remove(pos);
            let parent_v = parent.values.remove(pos);
            parent.children.remove(pos + 1);
            
            child.keys.push(parent_k);
            child.values.push(parent_v);
            
            child.keys.append(&mut right_sibling.keys);
            child.values.append(&mut right_sibling.values);
            
            if !right_sibling.is_leaf() {
                for &c_id in &right_sibling.children {
                    let mut c_node = self.get_node(c_id);
                    c_node.parent = Some(child.id);
                    self.save_node(&c_node);
                }
                child.children.append(&mut right_sibling.children);
            }
            
            self.save_node(&child);
            self.save_node(&parent);
            
            if let Some(gp_id) = parent.parent {
                if parent.keys.len() < min_keys {
                    self.rebalance(gp_id, parent.id);
                }
            }
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
            if parent.is_overflowing(self.max_keys) {
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

    pub fn search_path(&self, key: u64) -> Vec<serde_json::Value> {
        let mut path = Vec::new();
        self.search_path_recursive(self.root_id, key, &mut path);
        path
    }

    fn search_path_recursive(&self, node_id: u32, key: u64, path: &mut Vec<serde_json::Value>) {
        let node = self.get_node(node_id);
        
        let found = node.keys.binary_search(&key).is_ok();
        
        path.push(serde_json::json!({
            "id": node_id,
            "type": if node.is_leaf() { "leaf" } else { "internal" },
            "keys": node.keys,
            "found": found
        }));
        
        if !node.is_leaf() {
            match node.keys.binary_search(&key) {
                Ok(_) => {
                    // Key found in internal node. No need to recurse for search path.
                }
                Err(pos) => {
                    // Not found, go to appropriate child
                    if let Some(&child_id) = node.children.get(pos) {
                        self.search_path_recursive(child_id, key, path);
                    }
                }
            }
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
