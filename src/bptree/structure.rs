use std::rc::Rc;
use std::cell::RefCell;

pub(super) type NodePtr<K, V> = Rc<RefCell<Node<K, V>>>;

pub enum Node<K, V> {
    Internal(InternalNode<K, V>),
    Leaf(LeafNode<K, V>)
}

pub struct InternalNode<K, V> {
    pub keys: Vec<K>,
    pub children: Vec<NodePtr<K, V>>
}

pub struct LeafNode<K, V> {
    pub keys: Vec<K>,
    pub values: Vec<V>,

    pub next: Option<NodePtr<K, V>>
}

pub struct BPlusTree<K, V> {
    pub(super) root: Option<NodePtr<K, V>>,
    pub(super) degree: usize
}

impl<K: Ord, V: Clone> BPlusTree<K, V> {
    pub fn get(&self, key: &K) -> Option<V> {
        self.root.as_ref().and_then(|root| self.search_recursive(root.clone(), key))
    }

    fn search_recursive(&self, node_ptr: NodePtr<K, V>, key: &K) -> Option<V> {
        let node = node_ptr.borrow();

        match &*node {
            Node::Internal(internal) => {
                let idx= internal.keys.binary_search(key).unwrap_or_else(|e| e);
                self.search_recursive(internal.children[idx].clone(), key)
            }

            Node::Leaf(leaf) => {
                if let Ok(idx) = leaf.keys.binary_search(key) {
                    Some(leaf.values[idx].clone())
                } else {
                    None
                }
            }
        }
    }
}
