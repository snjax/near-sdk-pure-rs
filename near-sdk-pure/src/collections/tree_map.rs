use borsh::{BorshDeserialize, BorshSerialize};
use core::ops::Bound;

use crate::collections::LookupMap;
use crate::collections::{append, Vector};
use alloc::vec::Vec;

/// TreeMap based on AVL-tree
///
/// Runtime complexity (worst case):
/// - `get`/`contains_key`:     O(1) - UnorderedMap lookup
/// - `insert`/`remove`:        O(log(N))
/// - `min`/`max`:              O(log(N))
/// - `above`/`below`:          O(log(N))
/// - `range` of K elements:    O(Klog(N))
///
#[derive(BorshSerialize, BorshDeserialize)]
pub struct TreeMap<K, V> {
    root: u64,
    val: LookupMap<K, V>,
    tree: Vector<Node<K>>,
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct Node<K> {
    id: u64,
    key: K,           // key stored in a node
    lft: Option<u64>, // left link of a node
    rgt: Option<u64>, // right link of a node
    ht: u64,          // height of a subtree at a node
}

impl<K> Node<K>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
{
    fn of(id: u64, key: K) -> Self {
        Self { id, key, lft: None, rgt: None, ht: 1 }
    }
}

impl<K, V> TreeMap<K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    pub fn new(id: Vec<u8>) -> Self {
        Self {
            root: 0,
            val: LookupMap::new(append(&id, b'v')),
            tree: Vector::new(append(&id, b'n')),
        }
    }

    pub fn len(&self) -> u64 {
        self.tree.len() as u64
    }

    pub fn clear(&mut self) {
        self.root = 0;
        for n in self.tree.iter() {
            self.val.remove(&n.key);
        }
        self.tree.clear();
    }

    fn node(&self, id: u64) -> Option<Node<K>> {
        self.tree.get(id)
    }

    fn save(&mut self, node: &Node<K>) {
        if node.id < self.len() {
            self.tree.replace(node.id, node);
        } else {
            self.tree.push(node);
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.val.get(key).is_some()
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.val.get(key)
    }

    pub fn insert(&mut self, key: &K, val: &V) -> Option<V> {
        if !self.contains_key(&key) {
            self.root = self.insert_at(self.root, self.len(), &key);
        }
        self.val.insert(&key, &val)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if self.contains_key(&key) {
            self.root = self.do_remove(&key);
            self.val.remove(&key)
        } else {
            // no such key, nothing to do
            None
        }
    }

    /// Returns the smallest stored key from the tree
    pub fn min(&self) -> Option<K> {
        self.min_at(self.root, self.root).map(|(n, _)| n.key)
    }

    /// Returns the largest stored key from the tree
    pub fn max(&self) -> Option<K> {
        self.max_at(self.root, self.root).map(|(n, _)| n.key)
    }

    /// Returns the smallest key that is strictly greater than key given as the parameter
    pub fn higher(&self, key: &K) -> Option<K> {
        self.above_at(self.root, key)
    }

    /// Returns the largest key that is strictly less than key given as the parameter
    pub fn lower(&self, key: &K) -> Option<K> {
        self.below_at(self.root, key)
    }

    /// Returns the smallest key that is greater or equal to key given as the parameter
    pub fn ceil_key(&self, key: &K) -> Option<K> {
        if self.contains_key(key) {
            Some(key.clone())
        } else {
            self.higher(key)
        }
    }

    /// Returns the largest key that is less or equal to key given as the parameter
    pub fn floor_key(&self, key: &K) -> Option<K> {
        if self.contains_key(key) {
            Some(key.clone())
        } else {
            self.lower(key)
        }
    }

    /// Iterate all entries in ascending order: min to max, both inclusive
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::asc(&self).into_iter()
    }

    /// Iterate entries in ascending order: given key (exclusive) to max (inclusive)
    pub fn iter_from<'a>(&'a self, key: K) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::asc_from(&self, key).into_iter()
    }

    /// Iterate all entries in descending order: max to min, both inclusive
    pub fn iter_rev<'a>(&'a self) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::desc(&self).into_iter()
    }

    /// Iterate entries in descending order: given key (exclusive) to min (inclusive)
    pub fn iter_rev_from<'a>(&'a self, key: K) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::desc_from(&self, key).into_iter()
    }

    /// Iterate entries in ascending order according to specified bounds.
    ///
    /// # Panics
    ///
    /// Panics if range start > end.
    /// Panics if range start == end and both bounds are Excluded.
    pub fn range<'a>(&'a self, r: (Bound<K>, Bound<K>)) -> impl Iterator<Item = (K, V)> + 'a {
        let (lo, hi) = match r {
            (Bound::Included(a), Bound::Included(b)) if a > b => panic!("Invalid range."),
            (Bound::Excluded(a), Bound::Included(b)) if a > b => panic!("Invalid range."),
            (Bound::Included(a), Bound::Excluded(b)) if a > b => panic!("Invalid range."),
            (Bound::Excluded(a), Bound::Excluded(b)) if a == b => panic!("Invalid range."),
            (lo, hi) => (lo, hi),
        };

        Cursor::range(&self, lo, hi).into_iter()
    }

    pub fn to_vec(&self) -> Vec<(K, V)> {
        self.iter().collect()
    }

    //
    // Internal utilities
    //

    /// Returns (node, parent node) of left-most lower (min) node starting from given node `at`.
    /// As min_at only traverses the tree down, if a node `at` is the minimum node in a subtree,
    /// its parent must be explicitly provided in advance.
    fn min_at(&self, mut at: u64, p: u64) -> Option<(Node<K>, Node<K>)> {
        let mut parent: Option<Node<K>> = self.node(p);
        loop {
            let node = self.node(at);
            match node.clone().and_then(|n| n.lft) {
                Some(lft) => {
                    at = lft;
                    parent = node;
                }
                None => {
                    return node.and_then(|n| parent.map(|p| (n, p)));
                }
            }
        }
    }

    /// Returns (node, parent node) of right-most lower (max) node starting from given node `at`.
    /// As min_at only traverses the tree down, if a node `at` is the minimum node in a subtree,
    /// its parent must be explicitly provided in advance.
    fn max_at(&self, mut at: u64, p: u64) -> Option<(Node<K>, Node<K>)> {
        let mut parent: Option<Node<K>> = self.node(p);
        loop {
            let node = self.node(at);
            match node.clone().and_then(|n| n.rgt) {
                Some(rgt) => {
                    parent = node;
                    at = rgt;
                }
                None => {
                    return node.and_then(|n| parent.map(|p| (n, p)));
                }
            }
        }
    }

    fn above_at(&self, mut at: u64, key: &K) -> Option<K> {
        let mut seen: Option<K> = None;
        loop {
            let node = self.node(at);
            match node.clone().map(|n| n.key) {
                Some(k) => {
                    if k.le(key) {
                        match node.and_then(|n| n.rgt) {
                            Some(rgt) => at = rgt,
                            None => break,
                        }
                    } else {
                        seen = Some(k);
                        match node.and_then(|n| n.lft) {
                            Some(lft) => at = lft,
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }
        seen
    }

    fn below_at(&self, mut at: u64, key: &K) -> Option<K> {
        let mut seen: Option<K> = None;
        loop {
            let node = self.node(at);
            match node.clone().map(|n| n.key) {
                Some(k) => {
                    if k.lt(key) {
                        seen = Some(k);
                        match node.and_then(|n| n.rgt) {
                            Some(rgt) => at = rgt,
                            None => break,
                        }
                    } else {
                        match node.and_then(|n| n.lft) {
                            Some(lft) => at = lft,
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }
        seen
    }

    fn insert_at(&mut self, at: u64, id: u64, key: &K) -> u64 {
        match self.node(at) {
            None => {
                self.save(&Node::of(id, key.clone()));
                at
            }
            Some(mut node) => {
                if key.eq(&node.key) {
                    at
                } else {
                    if key.lt(&node.key) {
                        let idx = match node.lft {
                            Some(lft) => self.insert_at(lft, id, key),
                            None => self.insert_at(id, id, key),
                        };
                        node.lft = Some(idx);
                    } else {
                        let idx = match node.rgt {
                            Some(rgt) => self.insert_at(rgt, id, key),
                            None => self.insert_at(id, id, key),
                        };
                        node.rgt = Some(idx);
                    };

                    self.update_height(&mut node);
                    self.enforce_balance(&mut node)
                }
            }
        }
    }

    // Calculate and save the height of a subtree at node `at`:
    // height[at] = 1 + max(height[at.L], height[at.R])
    fn update_height(&mut self, node: &mut Node<K>) {
        let lft = node.lft.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();
        let rgt = node.rgt.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();

        node.ht = 1 + core::cmp::max(lft, rgt);
        self.save(&node);
    }

    // Balance = difference in heights between left and right subtrees at given node.
    fn get_balance(&self, node: &Node<K>) -> i64 {
        let lht = node.lft.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();
        let rht = node.rgt.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();

        lht as i64 - rht as i64
    }

    // Left rotation of an AVL subtree with at node `at`.
    // New root of subtree is returned, caller is responsible for updating proper link from parent.
    fn rotate_left(&mut self, node: &mut Node<K>) -> u64 {
        let mut lft = node.lft.and_then(|id| self.node(id)).unwrap();
        let lft_rgt = lft.rgt;

        // at.L = at.L.R
        node.lft = lft_rgt;

        // at.L.R = at
        lft.rgt = Some(node.id);

        // at = at.L
        self.update_height(node);
        self.update_height(&mut lft);

        lft.id
    }

    // Right rotation of an AVL subtree at node in `at`.
    // New root of subtree is returned, caller is responsible for updating proper link from parent.
    fn rotate_right(&mut self, node: &mut Node<K>) -> u64 {
        let mut rgt = node.rgt.and_then(|id| self.node(id)).unwrap();
        let rgt_lft = rgt.lft;

        // at.R = at.R.L
        node.rgt = rgt_lft;

        // at.R.L = at
        rgt.lft = Some(node.id);

        // at = at.R
        self.update_height(node);
        self.update_height(&mut rgt);

        rgt.id
    }

    // Check balance at a given node and enforce it if necessary with respective rotations.
    fn enforce_balance(&mut self, node: &mut Node<K>) -> u64 {
        let balance = self.get_balance(&node);
        if balance > 1 {
            let mut lft = node.lft.and_then(|id| self.node(id)).unwrap();
            if self.get_balance(&lft) < 0 {
                let rotated = self.rotate_right(&mut lft);
                node.lft = Some(rotated);
            }
            self.rotate_left(node)
        } else if balance < -1 {
            let mut rgt = node.rgt.and_then(|id| self.node(id)).unwrap();
            if self.get_balance(&rgt) > 0 {
                let rotated = self.rotate_left(&mut rgt);
                node.rgt = Some(rotated);
            }
            self.rotate_right(node)
        } else {
            node.id
        }
    }

    // Returns (node, parent node) for a node that holds the `key`.
    // For root node, same node is returned for node and parent node.
    fn lookup_at(&self, mut at: u64, key: &K) -> Option<(Node<K>, Node<K>)> {
        let mut p: Node<K> = self.node(at).unwrap();
        loop {
            match self.node(at) {
                Some(node) => {
                    if node.key.eq(key) {
                        return Some((node, p));
                    } else if node.key.lt(key) {
                        match node.rgt {
                            Some(rgt) => {
                                p = node;
                                at = rgt;
                            }
                            None => break,
                        }
                    } else {
                        match node.lft {
                            Some(lft) => {
                                p = node;
                                at = lft;
                            }
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }
        None
    }

    // Navigate from root to node holding `key` and backtrace back to the root
    // enforcing balance (if necessary) along the way.
    fn check_balance(&mut self, at: u64, key: &K) -> u64 {
        match self.node(at) {
            Some(mut node) => {
                if node.key.eq(key) {
                    self.update_height(&mut node);
                    self.enforce_balance(&mut node)
                } else {
                    if node.key.gt(key) {
                        match node.lft {
                            Some(l) => {
                                let id = self.check_balance(l, key);
                                node.lft = Some(id);
                            }
                            None => (),
                        }
                    } else {
                        match node.rgt {
                            Some(r) => {
                                let id = self.check_balance(r, key);
                                node.rgt = Some(id);
                            }
                            None => (),
                        }
                    }
                    self.update_height(&mut node);
                    self.enforce_balance(&mut node)
                }
            }
            None => at,
        }
    }

    // Node holding the key is not removed from the tree - instead the substitute node is found,
    // the key is copied to 'removed' node from substitute node, and then substitute node gets
    // removed from the tree.
    //
    // The substitute node is either:
    // - right-most (max) node of the left subtree (containing smaller keys) of node holding `key`
    // - or left-most (min) node of the right subtree (containing larger keys) of node holding `key`
    //
    fn do_remove(&mut self, key: &K) -> u64 {
        // r_node - node containing key of interest
        // p_node - immediate parent node of r_node
        let (mut r_node, mut p_node) = match self.lookup_at(self.root, key) {
            Some(x) => x,
            None => return self.root, // cannot remove a missing key, no changes to the tree needed
        };

        let lft_opt = r_node.lft;
        let rgt_opt = r_node.rgt;

        if lft_opt.is_none() && rgt_opt.is_none() {
            // remove leaf
            if p_node.key.lt(key) {
                p_node.rgt = None;
            } else {
                p_node.lft = None;
            }
            self.update_height(&mut p_node);

            self.swap_with_last(r_node.id);

            // removing node might have caused a imbalance - balance the tree up to the root,
            // starting from lowest affected key - the parent of a leaf node in this case
            self.check_balance(self.root, &p_node.key)
        } else {
            // non-leaf node, select subtree to proceed with
            let b = self.get_balance(&r_node);
            if b >= 0 {
                // proceed with left subtree
                let lft = lft_opt.unwrap();

                // k - max key from left subtree
                // n - node that holds key k, p - immediate parent of n
                let (n, mut p) = self.max_at(lft, r_node.id).unwrap();
                let k = n.key.clone();

                if p.rgt.clone().map(|id| id == n.id).unwrap_or_default() {
                    // n is on right link of p
                    p.rgt = n.lft;
                } else {
                    // n is on left link of p
                    p.lft = n.lft;
                }

                self.update_height(&mut p);

                if r_node.id == p.id {
                    // r_node.id and p.id can overlap on small trees (2 levels, 2-3 nodes)
                    // that leads to nasty lost update of the key, refresh below fixes that
                    r_node = self.node(r_node.id).unwrap();
                }
                r_node.key = k;
                self.save(&r_node);

                self.swap_with_last(n.id);

                // removing node might have caused an imbalance - balance the tree up to the root,
                // starting from the lowest affected key (max key from left subtree in this case)
                self.check_balance(self.root, &p.key)
            } else {
                // proceed with right subtree
                let rgt = rgt_opt.unwrap();

                // k - min key from right subtree
                // n - node that holds key k, p - immediate parent of n
                let (n, mut p) = self.min_at(rgt, r_node.id).unwrap();
                let k = n.key.clone();

                if p.lft.map(|id| id == n.id).unwrap_or_default() {
                    // n is on left link of p
                    p.lft = n.rgt;
                } else {
                    // n is on right link of p
                    p.rgt = n.rgt;
                }

                self.update_height(&mut p);

                if r_node.id == p.id {
                    // r_node.id and p.id can overlap on small trees (2 levels, 2-3 nodes)
                    // that leads to nasty lost update of the key, refresh below fixes that
                    r_node = self.node(r_node.id).unwrap();
                }
                r_node.key = k;
                self.save(&r_node);

                self.swap_with_last(n.id);

                // removing node might have caused a imbalance - balance the tree up to the root,
                // starting from the lowest affected key (min key from right subtree in this case)
                self.check_balance(self.root, &p.key)
            }
        }
    }

    // Move content of node with id = `len - 1` (parent left or right link, left, right, key, height)
    // to node with given `id`, and remove node `len - 1` (pop the vector of nodes).
    // This ensures that among `n` nodes in the tree, max `id` is `n-1`, so when new node is inserted,
    // it gets an `id` as its position in the vector.
    fn swap_with_last(&mut self, id: u64) {
        if id == self.len() - 1 {
            // noop: id is already last element in the vector
            self.tree.pop();
            return;
        }

        let key = self.node(self.len() - 1).map(|n| n.key).unwrap();
        let (mut n, mut p) = self.lookup_at(self.root, &key).unwrap();

        if n.id != p.id {
            if p.lft.map(|id| id == n.id).unwrap_or_default() {
                p.lft = Some(id);
            } else {
                p.rgt = Some(id);
            }
            self.save(&p);
        }

        if self.root == n.id {
            self.root = id;
        }

        n.id = id;
        self.save(&n);
        self.tree.pop();
    }
}

impl<'a, K, V> IntoIterator for &'a TreeMap<K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);
    type IntoIter = Cursor<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Cursor::asc(self)
    }
}

impl<K, V> Iterator for Cursor<'_, K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let this_key = self.key.clone();

        let next_key = self
            .key
            .take()
            .and_then(|k| if self.asc { self.map.higher(&k) } else { self.map.lower(&k) })
            .filter(|k| fits(k, &self.lo, &self.hi));
        self.key = next_key;

        this_key.and_then(|k| self.map.get(&k).map(|v| (k, v)))
    }
}

fn fits<K: Ord>(key: &K, lo: &Bound<K>, hi: &Bound<K>) -> bool {
    (match lo {
        Bound::Included(ref x) => key >= x,
        Bound::Excluded(ref x) => key > x,
        Bound::Unbounded => true,
    }) && (match hi {
        Bound::Included(ref x) => key <= x,
        Bound::Excluded(ref x) => key < x,
        Bound::Unbounded => true,
    })
}

pub struct Cursor<'a, K, V> {
    asc: bool,
    lo: Bound<K>,
    hi: Bound<K>,
    key: Option<K>,
    map: &'a TreeMap<K, V>,
}

impl<'a, K, V> Cursor<'a, K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    fn asc(map: &'a TreeMap<K, V>) -> Self {
        let key: Option<K> = map.min();
        Self { asc: true, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn asc_from(map: &'a TreeMap<K, V>, key: K) -> Self {
        let key = map.higher(&key);
        Self { asc: true, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn desc(map: &'a TreeMap<K, V>) -> Self {
        let key: Option<K> = map.max();
        Self { asc: false, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn desc_from(map: &'a TreeMap<K, V>, key: K) -> Self {
        let key = map.lower(&key);
        Self { asc: false, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn range(map: &'a TreeMap<K, V>, lo: Bound<K>, hi: Bound<K>) -> Self {
        let key = match &lo {
            Bound::Included(k) if map.contains_key(k) => Some(k.clone()),
            Bound::Included(k) | Bound::Excluded(k) => map.higher(k),
            _ => None,
        };
        let key = key.filter(|k| fits(k, &lo, &hi));

        Self { asc: true, key, lo, hi, map }
    }
}
