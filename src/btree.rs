use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use serde::{Deserialize, Serialize};
use crate::pager::Pager;
use crate::node::{Node, NodeType, LeafNode, InternalNode};

/// Simple file logger for debugging B-tree operations
pub struct Logger;

impl Logger {
    fn log(msg: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] {}\n", timestamp, msg);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("btree_debug.log")
        {
            let _ = file.write_all(log_line.as_bytes());
        }
    }

    pub fn info(msg: &str) {
        Self::log(&format!("[INFO] {}", msg));
    }

    pub fn error(msg: &str) {
        Self::log(&format!("[ERROR] {}", msg));
    }

    pub fn operation(op: &str, details: &str) {
        Self::log(&format!("[OP:{}] {}", op, details));
    }

    pub fn node_state(node_id: u32, node_type: &str, keys: &[u64], children: &[u32], parent: Option<u32>) {
        let parent_str = match parent {
            Some(p) => format!("parent={}", p),
            None => "parent=None".to_string(),
        };
        Self::log(&format!(
            "[NODE:{}] type={} keys={:?} children={:?} {}",
            node_id, node_type, keys, children, parent_str
        ));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaPage {
    pub root_id: u32,
    pub max_keys: usize,
}

pub struct BTree {
    pub pager: Arc<Mutex<Pager>>,
    pub root_id: u32,
    pub max_keys: usize,
}

impl BTree {
    pub fn new(pager: Arc<Mutex<Pager>>, max_keys: usize) -> Self {
        let mut p = pager.lock().unwrap();

        // Check if DB is empty / new
        if p.num_pages == 1 {
            // Allocate root node
            let root_id = p.allocate_page();
            let root_node = Node::new_leaf(root_id);
            p.write_page(root_id, &root_node.serialize()).unwrap();

            // Write meta page
            let meta = MetaPage { root_id, max_keys };
            let meta_bytes = bincode::serialize(&meta).unwrap();
            p.write_page(0, &meta_bytes).unwrap();

            drop(p);
            return Self { pager, root_id, max_keys };
        }

        let meta_bytes = p.read_page(0).unwrap();
        // Ignore trailing zeros from the page buffer
        let meta: MetaPage = bincode::deserialize(&meta_bytes).unwrap();
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
        let root_node = Node::new_leaf(root_id);
        p.write_page(root_id, &root_node.serialize()).unwrap();

        let meta = MetaPage { root_id, max_keys };
        let meta_bytes = bincode::serialize(&meta).unwrap();
        p.write_page(0, &meta_bytes).unwrap();
        self.root_id = root_id;
    }

    /// Standard B-tree min keys: ceiling(max_keys / 2)
    /// For order n (where max_keys = n-1): min_keys = ⌈(n-1)/2⌉ = ceiling(max_keys / 2)
    pub fn calc_min_keys(&self) -> usize {
        (self.max_keys + 1) / 2
    }

    pub fn get_node(&self, id: u32) -> Node {
        let mut p = self.pager.lock().unwrap();
        let bytes = p.read_page(id).unwrap();
        Node::deserialize(&bytes).unwrap()
    }

    pub fn save_node(&self, node: &Node) {
        let (keys, children, parent, node_type) = match &node.node_type {
            NodeType::Leaf(l) => (l.keys.clone(), vec![], node.parent, "Leaf"),
            NodeType::Internal(i) => (i.keys.clone(), i.children.clone(), node.parent, "Internal"),
        };
        Logger::node_state(node.id, node_type, &keys, &children, parent);
        let mut p = self.pager.lock().unwrap();
        p.write_page(node.id, &node.serialize()).unwrap();
    }

    fn save_meta(&mut self) {
        let meta = MetaPage { root_id: self.root_id, max_keys: self.max_keys };
        let meta_bytes = bincode::serialize(&meta).unwrap();
        let mut p = self.pager.lock().unwrap();
        p.write_page(0, &meta_bytes).unwrap();
    }

    pub fn insert(&mut self, key: u64, value: String) {
        Logger::operation("INSERT", &format!("key={} root_id={}", key, self.root_id));
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

                        if node.is_overflowing(self.max_keys) {
                            self.split_node(node);
                        }
                    }
                }
            }
            NodeType::Internal(internal) => {
                let pos = match internal.keys.binary_search(&key) {
                    Ok(pos) => pos + 1,
                    Err(pos) => pos,
                };
                let child_id = internal.children[pos];
                let child = self.get_node(child_id);
                self.insert_into_node(child, key, value);
            }
        }
    }

    pub fn delete(&mut self, key: u64) {
        Logger::operation("DELETE", &format!("key={} root_id={}", key, self.root_id));
        let root = self.get_node(self.root_id);
        self.delete_from_node(root, key);

        let mut root = self.get_node(self.root_id);
        match &root.node_type {
            NodeType::Internal(internal) => {
                if internal.keys.is_empty() {
                    Logger::operation("ROOT_SHRINK", &format!("old_root={} new_root={}", self.root_id, internal.children[0]));
                    self.root_id = internal.children[0];
                    let mut new_root = self.get_node(self.root_id);
                    new_root.parent = None;
                    self.save_node(&new_root);
                    self.save_meta();
                }
            }
            _ => {}
        }
    }

    fn delete_from_node(&mut self, mut node: Node, key: u64) {
        match &mut node.node_type {
            NodeType::Leaf(leaf) => {
                if let Ok(pos) = leaf.keys.binary_search(&key) {
                    let old_first_key = leaf.keys[0];
                    leaf.keys.remove(pos);
                    leaf.values.remove(pos);
                    
                    let mut new_first_key = None;
                    if pos == 0 && !leaf.keys.is_empty() {
                        new_first_key = Some(leaf.keys[0]);
                    }
                    
                    self.save_node(&node);
                    
                    if let Some(nfk) = new_first_key {
                        self.replace_routing_key(node.id, old_first_key, nfk);
                    }
                }
            }
            NodeType::Internal(internal) => {
                let pos = match internal.keys.binary_search(&key) {
                    Ok(pos) => pos + 1,
                    Err(pos) => pos,
                };
                let child_id = internal.children[pos];
                let child = self.get_node(child_id);
                self.delete_from_node(child, key);
            }
        }
        
        let node = self.get_node(node.id);
        if let Some(parent_id) = node.parent {
            let min_keys = self.calc_min_keys();
            let len = match &node.node_type {
                NodeType::Leaf(leaf) => leaf.keys.len(),
                NodeType::Internal(internal) => internal.keys.len(),
            };
            if len < min_keys {
                Logger::operation("UNDERFLOW", &format!("node={} len={} min_keys={} parent={}", node.id, len, min_keys, parent_id));
                self.rebalance(parent_id, node.id);
            }
        }
    }
    
    fn replace_routing_key(&mut self, mut node_id: u32, old_key: u64, new_key: u64) {
        while let Some(parent_id) = self.get_node(node_id).parent {
            let mut parent = self.get_node(parent_id);
            if let NodeType::Internal(internal) = &mut parent.node_type {
                if let Some(pos) = internal.keys.iter().position(|&k| k == old_key) {
                    internal.keys[pos] = new_key;
                    self.save_node(&parent);
                    return;
                }
            }
            node_id = parent_id;
        }
    }

    fn rebalance(&mut self, parent_id: u32, child_id: u32) {
        Logger::operation("REBALANCE_START", &format!("parent={} child={}", parent_id, child_id));
        let mut parent = self.get_node(parent_id);
        let pos = match &parent.node_type {
            NodeType::Internal(internal) => {
                match internal.children.iter().position(|&id| id == child_id) {
                    Some(p) => {
                        Logger::operation("REBALANCE_POS", &format!("child={} found at pos={} in parent={} children={:?}", child_id, p, parent_id, internal.children));
                        p
                    }
                    None => {
                        let msg = format!("Child {} not found in parent {}'s children {:?}", child_id, parent_id, internal.children);
                        Logger::error(&msg);
                        eprintln!("ERROR: {}", msg);
                        return; // Skip rebalancing rather than panic
                    }
                }
            }
            _ => {
                let msg = format!("Parent {} is not an internal node", parent_id);
                Logger::error(&msg);
                eprintln!("ERROR: {}", msg);
                return;
            }
        };

        let min_keys = self.calc_min_keys();
        let is_leaf = self.get_node(child_id).is_leaf();
        Logger::operation("REBALANCE_CHECK", &format!("min_keys={} is_leaf={} pos={}", min_keys, is_leaf, pos));

        if is_leaf {
            if pos > 0 {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut left_sibling = self.get_node(p_internal.children[pos - 1]);
                let l_len = match &left_sibling.node_type { NodeType::Leaf(l) => l.keys.len(), _ => unreachable!() };
                Logger::operation("REBALANCE_TRY_BORROW_LEFT", &format!("child={} sibling={} sibling_keys={} min_keys={}", child_id, left_sibling.id, l_len, min_keys));
                if l_len > min_keys {
                    let mut child = self.get_node(child_id);
                    if let (NodeType::Leaf(left_l), NodeType::Leaf(child_l)) = (&mut left_sibling.node_type, &mut child.node_type) {
                        let borrow_k = left_l.keys.pop().unwrap();
                        let borrow_v = left_l.values.pop().unwrap();
                        Logger::operation("BORROW_LEFT", &format!("child={} borrowed key={} from sibling={}", child_id, borrow_k, left_sibling.id));
                        child_l.keys.insert(0, borrow_k);
                        child_l.values.insert(0, borrow_v);
                        p_internal.keys[pos - 1] = borrow_k;
                    }
                    self.save_node(&left_sibling);
                    self.save_node(&child);
                    self.save_node(&parent);
                    return;
                }
            }
            if pos < match &parent.node_type { NodeType::Internal(i) => i.children.len() - 1, _ => 0 } {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut right_sibling = self.get_node(p_internal.children[pos + 1]);
                let r_len = match &right_sibling.node_type { NodeType::Leaf(l) => l.keys.len(), _ => unreachable!() };
                Logger::operation("REBALANCE_TRY_BORROW_RIGHT", &format!("child={} sibling={} sibling_keys={} min_keys={}", child_id, right_sibling.id, r_len, min_keys));
                if r_len > min_keys {
                    let mut child = self.get_node(child_id);
                    if let (NodeType::Leaf(right_l), NodeType::Leaf(child_l)) = (&mut right_sibling.node_type, &mut child.node_type) {
                        let borrow_k = right_l.keys.remove(0);
                        let borrow_v = right_l.values.remove(0);
                        Logger::operation("BORROW_RIGHT", &format!("child={} borrowed key={} from sibling={}", child_id, borrow_k, right_sibling.id));
                        child_l.keys.push(borrow_k);
                        child_l.values.push(borrow_v);
                        p_internal.keys[pos] = right_l.keys[0];
                    }
                    self.save_node(&right_sibling);
                    self.save_node(&child);
                    self.save_node(&parent);
                    return;
                }
            }
            if pos > 0 {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut left_sibling = self.get_node(p_internal.children[pos - 1]);
                let mut child = self.get_node(child_id);
                Logger::operation("MERGE_LEFT", &format!("child={} merged into sibling={} parent={}", child_id, left_sibling.id, parent_id));
                p_internal.keys.remove(pos - 1);
                p_internal.children.remove(pos);
                if let (NodeType::Leaf(left_l), NodeType::Leaf(child_l)) = (&mut left_sibling.node_type, &mut child.node_type) {
                    left_l.keys.append(&mut child_l.keys);
                    left_l.values.append(&mut child_l.values);
                    left_l.next_leaf = child_l.next_leaf;
                }
                self.save_node(&left_sibling);
                self.save_node(&parent);
                if let Some(gp_id) = parent.parent {
                    if match &parent.node_type { NodeType::Internal(i) => i.keys.len() < min_keys, _ => false } {
                        Logger::operation("REBALANCE_RECURSIVE", &format!("parent={} underflow, rebalancing with grandparent={}", parent_id, gp_id));
                        self.rebalance(gp_id, parent.id);
                    }
                }
            } else {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut right_sibling = self.get_node(p_internal.children[pos + 1]);
                let mut child = self.get_node(child_id);
                Logger::operation("MERGE_RIGHT", &format!("sibling={} merged into child={} parent={}", right_sibling.id, child_id, parent_id));
                p_internal.keys.remove(pos);
                p_internal.children.remove(pos + 1);
                if let (NodeType::Leaf(child_l), NodeType::Leaf(right_l)) = (&mut child.node_type, &mut right_sibling.node_type) {
                    child_l.keys.append(&mut right_l.keys);
                    child_l.values.append(&mut right_l.values);
                    child_l.next_leaf = right_l.next_leaf;
                }
                self.save_node(&child);
                self.save_node(&parent);
                if let Some(gp_id) = parent.parent {
                    if match &parent.node_type { NodeType::Internal(i) => i.keys.len() < min_keys, _ => false } {
                        Logger::operation("REBALANCE_RECURSIVE", &format!("parent={} underflow, rebalancing with grandparent={}", parent_id, gp_id));
                        self.rebalance(gp_id, parent.id);
                    }
                }
            }
        } else {
            // Internal node rebalancing
            Logger::operation("REBALANCE_INTERNAL", &format!("child={} is internal node", child_id));
            if pos > 0 {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut left_sibling = self.get_node(p_internal.children[pos - 1]);
                let l_len = match &left_sibling.node_type { NodeType::Internal(i) => i.keys.len(), _ => unreachable!() };
                Logger::operation("REBALANCE_TRY_BORROW_LEFT_INT", &format!("child={} sibling={} sibling_keys={} min_keys={}", child_id, left_sibling.id, l_len, min_keys));
                if l_len > min_keys {
                    let mut child = self.get_node(child_id);
                    if let (NodeType::Internal(left_i), NodeType::Internal(child_i)) = (&mut left_sibling.node_type, &mut child.node_type) {
                        let borrow_k = left_i.keys.pop().unwrap();
                        let borrow_c = left_i.children.pop().unwrap();
                        let parent_k = p_internal.keys[pos - 1];
                        Logger::operation("BORROW_LEFT_INT", &format!("child={} borrowed key={} child={} from sibling={}", child_id, borrow_k, borrow_c, left_sibling.id));
                        p_internal.keys[pos - 1] = borrow_k;
                        child_i.keys.insert(0, parent_k);
                        child_i.children.insert(0, borrow_c);

                        let mut bc_node = self.get_node(borrow_c);
                        bc_node.parent = Some(child.id);
                        self.save_node(&bc_node);
                    }
                    self.save_node(&left_sibling);
                    self.save_node(&child);
                    self.save_node(&parent);
                    return;
                }
            }
            if pos < match &parent.node_type { NodeType::Internal(i) => i.children.len() - 1, _ => 0 } {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut right_sibling = self.get_node(p_internal.children[pos + 1]);
                let r_len = match &right_sibling.node_type { NodeType::Internal(i) => i.keys.len(), _ => unreachable!() };
                Logger::operation("REBALANCE_TRY_BORROW_RIGHT_INT", &format!("child={} sibling={} sibling_keys={} min_keys={}", child_id, right_sibling.id, r_len, min_keys));
                if r_len > min_keys {
                    let mut child = self.get_node(child_id);
                    if let (NodeType::Internal(right_i), NodeType::Internal(child_i)) = (&mut right_sibling.node_type, &mut child.node_type) {
                        let borrow_k = right_i.keys.remove(0);
                        let borrow_c = right_i.children.remove(0);
                        let parent_k = p_internal.keys[pos];
                        Logger::operation("BORROW_RIGHT_INT", &format!("child={} borrowed key={} child={} from sibling={}", child_id, borrow_k, borrow_c, right_sibling.id));
                        p_internal.keys[pos] = borrow_k;
                        child_i.keys.push(parent_k);
                        child_i.children.push(borrow_c);

                        let mut bc_node = self.get_node(borrow_c);
                        bc_node.parent = Some(child.id);
                        self.save_node(&bc_node);
                    }
                    self.save_node(&right_sibling);
                    self.save_node(&child);
                    self.save_node(&parent);
                    return;
                }
            }
            if pos > 0 {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut left_sibling = self.get_node(p_internal.children[pos - 1]);
                let mut child = self.get_node(child_id);
                let parent_k = p_internal.keys.remove(pos - 1);
                p_internal.children.remove(pos);
                Logger::operation("MERGE_LEFT_INT", &format!("child={} merged into sibling={} parent={} parent_key={}", child_id, left_sibling.id, parent_id, parent_k));
                if let (NodeType::Internal(left_i), NodeType::Internal(child_i)) = (&mut left_sibling.node_type, &mut child.node_type) {
                    left_i.keys.push(parent_k);
                    left_i.keys.append(&mut child_i.keys);
                    left_i.children.append(&mut child_i.children);
                    for &c in &left_i.children {
                        let mut cn = self.get_node(c);
                        cn.parent = Some(left_sibling.id);
                        self.save_node(&cn);
                    }
                }
                self.save_node(&left_sibling);
                self.save_node(&parent);
                if let Some(gp_id) = parent.parent {
                    if match &parent.node_type { NodeType::Internal(i) => i.keys.len() < min_keys, _ => false } {
                        Logger::operation("REBALANCE_RECURSIVE", &format!("parent={} underflow after merge, rebalancing with grandparent={}", parent_id, gp_id));
                        self.rebalance(gp_id, parent.id);
                    }
                }
            } else {
                let mut p_internal = match &mut parent.node_type { NodeType::Internal(i) => i, _ => unreachable!() };
                let mut right_sibling = self.get_node(p_internal.children[pos + 1]);
                let mut child = self.get_node(child_id);
                let parent_k = p_internal.keys.remove(pos);
                p_internal.children.remove(pos + 1);
                Logger::operation("MERGE_RIGHT_INT", &format!("sibling={} merged into child={} parent={} parent_key={}", right_sibling.id, child_id, parent_id, parent_k));
                if let (NodeType::Internal(child_i), NodeType::Internal(right_i)) = (&mut child.node_type, &mut right_sibling.node_type) {
                    child_i.keys.push(parent_k);
                    child_i.keys.append(&mut right_i.keys);
                    child_i.children.append(&mut right_i.children);
                    for &c in &child_i.children {
                        let mut cn = self.get_node(c);
                        cn.parent = Some(child.id);
                        self.save_node(&cn);
                    }
                }
                self.save_node(&child);
                self.save_node(&parent);
                if let Some(gp_id) = parent.parent {
                    if match &parent.node_type { NodeType::Internal(i) => i.keys.len() < min_keys, _ => false } {
                        Logger::operation("REBALANCE_RECURSIVE", &format!("parent={} underflow after merge, rebalancing with grandparent={}", parent_id, gp_id));
                        self.rebalance(gp_id, parent.id);
                    }
                }
            }
        }
    }

    fn split_node(&mut self, mut node: Node) {
        let new_id = self.pager.lock().unwrap().allocate_page();
        Logger::operation("SPLIT", &format!("node={} new_id={}", node.id, new_id));
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
            Logger::operation("SPLIT_COMPLETE", &format!("node={} sibling={} parent={} up_key={}", node.id, sibling.id, parent_id, up_key));

            if let NodeType::Internal(p_internal) = &mut parent.node_type {
                let pos = p_internal.keys.binary_search(&up_key).unwrap_or_else(|e| e);
                p_internal.keys.insert(pos, up_key);
                p_internal.children.insert(pos + 1, sibling.id);
            }
            self.save_node(&parent);

            if parent.is_overflowing(self.max_keys) {
                self.split_node(parent);
            }
        } else {
            // Root split
            let new_root_id = self.pager.lock().unwrap().allocate_page();
            Logger::operation("ROOT_SPLIT", &format!("old_root={} new_root={} node={} sibling={} up_key={}", node.id, new_root_id, node.id, sibling.id, up_key));
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

    pub fn search_path(&self, key: u64) -> Vec<serde_json::Value> {
        let mut path = Vec::new();
        self.search_path_recursive(self.root_id, key, &mut path);
        path
    }

    fn search_path_recursive(&self, node_id: u32, key: u64, path: &mut Vec<serde_json::Value>) {
        let node = self.get_node(node_id);
        
        match &node.node_type {
            NodeType::Leaf(leaf) => {
                let found = leaf.keys.binary_search(&key).is_ok();
                path.push(serde_json::json!({
                    "id": node_id,
                    "type": "leaf",
                    "keys": leaf.keys,
                    "found": found
                }));
            }
            NodeType::Internal(internal) => {
                let pos = match internal.keys.binary_search(&key) {
                    Ok(pos) => pos + 1,
                    Err(pos) => pos,
                };
                let found_in_node = internal.keys.binary_search(&key).is_ok();
                path.push(serde_json::json!({
                    "id": node_id,
                    "type": "internal",
                    "keys": internal.keys,
                    "next_child": internal.children.get(pos).copied(),
                    "found": found_in_node
                }));
                if let Some(&child_id) = internal.children.get(pos) {
                    self.search_path_recursive(child_id, key, path);
                }
            }
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
