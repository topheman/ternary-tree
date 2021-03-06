/*!
A Rust implementation of Ternary Search Trees, with no unsafe blocks

A Ternary Search Tree (TST) is a data structure which stores key/value pairs in a tree. The key is a string, and its characters are placed in the tree nodes. Each node may have three children (hence the name) : a _left_ child, a _middle_ child and a _right_ child.

A search in a TST compares the current character in the key with the character of the current node :

* If both matches, the search traverse the middle child, and proceed to the next character in the key
* If the key character is less than the node one, the search simply goes through the left child, and keep looking for the same key character
* Respectively, if the key character is greater than the node one, the search simply goes through the right child

The data structure and its algorithm are explained very well in [Dr.Dobb's Ternary Search Trees](http://www.drdobbs.com/database/ternary-search-trees/184410528) article.

The following tree is the TST we get after inserting the following keys in order : "aba", "ab", "bc", "ac", "abc", "a", "b", "aca", "caa", "cbc", "bac", "c", "cca", "aab", "abb", "aa" (see `tst.dot` produced by code below)

![An example of a Ternary Search Tree](http://files.jmontmartin.net/crates_io_sample_tst.png "An example of a Ternary Search Tree")

A checked box "☑" denotes a node  which stores a value (it corresponds to the last character of a key). An empty box "☐" means that the node has no value.

A TST can be used as a map, but it allows more flexible ways to retrieve values associated with keys. This crate provides four ways to iterate over the values of a TST :

* get all values (same as a regular map), with `visit_values` or `iter`
* get all values whose keys begin with some prefix (i.e. _complete_ some prefix), with `visit_complete_values` or `iter_complete`
* get all values whose keys are _close_ to some string ([Hamming distance](https://en.wikipedia.org/wiki/Hamming_distance)), with `visit_neighbor_values` or `iter_neighbor`
* get all values whose keys match a string with some joker (e.g. "a?c"), with `visit_crossword_values` or `iter_crossword`

Visit methods are recursive and apply a closure to found values. They exist in immutable and mutable version (i.e. `visit_neighbor_values_mut`). But once a value is found (based on its key), they offer no way to know what the actual key is.

Iterators, on the other hand, save their context in a `Vec` and only work on immutable trees. However they are double ended, and support `next` and `next_back` methods to walk the tree from both ends. Moreover, once a value is found, they offer the `current_key` and `current_key_back` methods to retrieve the key associated with the last value.

The following lines may give you a foretaste of this crate and TSTs

```
extern crate ternary_tree;

use ternary_tree::Tst;
use std::fs::File;
use std::error::Error;

const SOME_KEYS : [&str; 16] = ["aba", "ab", "bc", "ac",
"abc", "a", "b", "aca", "caa", "cbc", "bac", "c", "cca",
"aab", "abb", "aa"];

let mut map = Tst::new();

for key in &SOME_KEYS {

    //Say the value is the same as the key,
    //it makes the example easier !
    let some_value = *key;

    map.insert(key, some_value);
}

//Use Graphviz to convert tst.dot to tst.png:
//dot -T png -o tst.png tst.dot
let mut file = File::create("tst.dot").unwrap();
map.pretty_print(&mut file);

let mut v = Vec::new();

//Recursively get all values whose keys match "a?a" pattern
map.visit_crossword_values("a?a", '?', |s| v.push(s.clone()));
assert_eq!(v, ["aba", "aca"]);

v.clear();

//Iterate over all values whose keys are close to "abc" (Hamming distance of 1)
{
    let mut it = map.iter_neighbor("abc", 1);

    while let Some(value) = it.next() {

        v.push(*value);
    }
    assert_eq!(v, ["ab", "aba", "abb", "abc", "cbc"]);

    v.clear();
}

//Mutate all values whose keys begin with "c"
map.visit_complete_values_mut("c", |s| *s = "xxx");

assert_eq!(map.get("caa"), Some(&"xxx"));
assert_eq!(map.get("cbc"), Some(&"xxx"));
assert_eq!(map.get("cca"), Some(&"xxx"));
```
*/

#![forbid(unsafe_code)]

use std::str::Chars;
use std::mem::replace;
use std::cmp::Ordering::Less;
use std::cmp::Ordering::Equal;
use std::cmp::Ordering::Greater;
use std::io::Write;
use std::ptr;
use std::fmt;
use std::mem;


pub struct Tst<T> {

    root: Link<T>,
    count: usize
}


type Link<T> = Option<Box<Node<T>>>;


struct Node<T> {

    label: char,
    value: Option<T>,
    left: Link<T>,
    middle: Link<T>,
    right: Link<T>
}


impl<T> Default for Node<T> {

    fn default() -> Node<T> {

        Node {

            label: '\0',
            value: None,
            left: None,
            middle: None,
            right: None
        }
    }
}


impl<T> fmt::Debug for Node<T> {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

            let value_box = match self.value {

                None => "☐", Some(_) => "☑"
            };

        write!(f, "{}-{}", value_box, self.label)
    }
}


fn insert_r<T>(link: &mut Link<T>, label: char, mut key_tail: Chars, value: T) -> Option<T> {

    let choose_branch_and_do_insert = |node: &mut Box<Node<T>>| match label.cmp(&node.label) {

        Less => insert_r(&mut node.left, label, key_tail, value),

        Greater => insert_r(&mut node.right, label, key_tail, value),

        Equal => {

            let new_label = key_tail.next();

            match new_label {

                None => replace(&mut node.value, Some(value)),

                Some(label) => insert_r(&mut node.middle, label, key_tail, value)
            }
        }
    };

    match link {

        None => {

            let mut node = Box::new(Node::<T>{label, .. Default::default()});

            let old_value = choose_branch_and_do_insert(&mut node);

            *link = Some(node);

            old_value
        }

        Some(ref mut node) => choose_branch_and_do_insert(node)
    }
}


fn get_r<'a, T>(link: &'a Link<T>, label: char, key_tail: &mut Chars) -> Option<&'a T> {

    match *link {

        None => None,

        Some(ref node) => match label.cmp(&node.label) {

            Less => get_r(&node.left, label, key_tail),

            Equal => {

                let new_label = key_tail.next();

                match new_label {

                    None => match node.value {

                        None => None,

                        Some(ref value) => Some(value)
                    }

                    Some(label) => get_r(&node.middle, label, key_tail)
                }
            },

            Greater => get_r(&node.right, label, key_tail),
        }
    }
}


fn get_r_mut<'a, T>(link: &'a mut Link<T>, label: char, key_tail: &mut Chars) -> Option<&'a mut T> {

    match *link {

        None => None,

        Some(ref mut node) => match label.cmp(&node.label) {

            Less => get_r_mut(&mut node.left, label, key_tail),

            Equal => {

                let new_label = key_tail.next();

                match new_label {

                    None => match node.value {

                        None => None,

                        Some(ref mut value) => Some(value)
                    }

                    Some(label) => get_r_mut(&mut node.middle, label, key_tail)
                }
            },

            Greater => get_r_mut(&mut node.right, label, key_tail),
        }
    }
}


fn remove_r<T>(link: &mut Link<T>, label: char, key_tail: &mut Chars) -> (bool, Option<T>) {

    match *link {

        None => (false, None),

        Some(ref mut node) => match label.cmp(&node.label) {

            Less => {

                let (prune, old_value) = remove_r(&mut node.left, label, key_tail);

                if prune {

                    node.left = None;
                }

                let more_pruning = node.value.is_none() && node.left.is_none() && node.middle.is_none() && node.right.is_none();
                (more_pruning, old_value)
            }

            Equal => {

                let new_label = key_tail.next();

                match new_label {

                    None => {

                        let old_value = replace(&mut node.value, None);

                        let prune = old_value.is_some() && node.left.is_none() && node.middle.is_none() && node.right.is_none();
                        (prune, old_value)
                    }

                    Some(label) => {

                        let (prune, old_value) = remove_r(&mut node.middle, label, key_tail);

                        if prune {

                            node.middle = None;
                        }

                        let more_pruning = node.value.is_none() && node.left.is_none() && node.middle.is_none() && node.right.is_none();
                        (more_pruning, old_value)
                    }
                }
            }

            Greater => {

                let (prune, old_value) = remove_r(&mut node.right, label, key_tail);

                if prune {

                    node.right = None;
                }

                let more_pruning = node.value.is_none() && node.left.is_none() && node.middle.is_none() && node.right.is_none();
                (more_pruning, old_value)
            }
        }
    }
}


#[derive(Default,PartialEq,Debug)]
pub struct DistStat { pub matches: usize, pub sides: usize, pub depth: usize }

#[derive(Default,PartialEq,Debug)]
pub struct KeyLenStat { pub min: usize, pub max: usize }

#[derive(Default,PartialEq,Debug)]
pub struct CountStat { pub nodes:usize, pub values: usize }

#[derive(Default,PartialEq,Debug)]
pub struct BytesStat { pub node: usize, pub total: usize }

#[derive(Default,PartialEq,Debug)]
pub struct Stats {

    pub dist: Vec<DistStat>,
    pub key_len: KeyLenStat,
    pub count: CountStat,
    pub bytes: BytesStat,
}


fn stat_r<T>(stats: Stats, link: &Link<T>, matches: usize, sides: usize, depth: usize) -> Stats {

    match *link {

        None => stats,

        Some(ref node) => {

            let mut stats = stat_r(stats, &node.left, matches, sides+1, depth+1);

            stats.count.nodes+=1;

            if node.value.is_some() {

                let matches = matches + 1;
                let depth = depth + 1;

                while stats.dist.len() <= depth {

                    stats.dist.push(DistStat { matches: 0, sides: 0, depth: 0 });
                }

                stats.dist[matches].matches+=1;
                stats.dist[sides].sides+=1;
                stats.dist[depth].depth+=1;

                if stats.key_len.min == 0 || matches < stats.key_len.min {

                    stats.key_len.min = matches;
                }

                if matches > stats.key_len.max {

                    stats.key_len.max = matches;
                }

                stats.count.values+=1;
            }

            let mut stats = stat_r(stats, &node.middle, matches+1, sides, depth+1);
            let stats = stat_r(stats, &node.right, matches, sides+1, depth+1);

            stats
        }
    }
}


//TODO - Documenter piège : le préfix à compléter est "passé" et l'éventuelle valeur attachée au préfix n'est par conséquent pas remontée
fn find_complete_root_r<'a, T>(link: &'a Link<T>, label: char, mut key_tail: Chars) -> &'a Link<T> {

    match *link {

        None => &link,

        Some(ref node) => match label.cmp(&node.label) {

            Less => find_complete_root_r(&node.left, label, key_tail),

            Greater => find_complete_root_r(&node.right, label, key_tail),

            Equal => {

                let new_label = key_tail.next();

                match new_label {

                    None => &node.middle,

                    Some(label) => find_complete_root_r(&node.middle, label, key_tail)
                }
            }
        }
    }
}


fn find_complete_root_r_mut<'a, T>(link: &'a mut Link<T>, label: char, mut key_tail: Chars) -> &'a mut Link<T> {

    match *link {

        None => { link }

        Some(ref mut node) => match label.cmp(&node.label) {

            Less => find_complete_root_r_mut(&mut node.left, label, key_tail),

            Greater => find_complete_root_r_mut(&mut node.right, label, key_tail),

            Equal => {

                let new_label = key_tail.next();

                match new_label {

                    None => &mut node.middle,

                    Some(label) => find_complete_root_r_mut(&mut node.middle, label, key_tail)
                }
            }
        }
    }
}


fn visit_values_r<'a, T, C>(link: &'a Link<T>, callback: &mut C)
where C: FnMut (&T) {

    match *link {

        None => return,

        Some(ref node) => {

            visit_values_r(&node.left, callback);

            if let Some(ref value) = node.value {

                callback(value);
            }

            visit_values_r(&node.middle, callback);
            visit_values_r(&node.right, callback);
        }
    }
}


fn visit_values_r_mut<'a, T, C>(link: &'a mut Link<T>, callback: &mut C)
where C: FnMut (&mut T) {

    match *link {

        None => return,

        Some(ref mut node) => {

            visit_values_r_mut(&mut node.left, callback);

            if let Some(ref mut value) = node.value {

                callback(value);
            }

            visit_values_r_mut(&mut node.middle, callback);
            visit_values_r_mut(&mut node.right, callback);
        }
    }
}


fn visit_complete_values_r<'a, T, C>(link: &'a Link<T>, callback: &mut C)
where C: FnMut (&T) {

    match *link {

        None => return,

        Some(ref node) => {

            visit_values_r(&node.left, callback);

            if let Some(ref value) = node.value {

                callback(value);
            }

            visit_values_r(&node.middle, callback);
            visit_values_r(&node.right, callback);
        }
    }
}


fn visit_complete_values_r_mut<'a, T, C>(link: &'a mut Link<T>, callback: &mut C)
where C: FnMut (&mut T) {

    match *link {

        None => return,

        Some(ref mut node) => {

            visit_values_r_mut(&mut node.left, callback);

            if let Some(ref mut value) = node.value {

                callback(value);
            }

            visit_values_r_mut(&mut node.middle, callback);
            visit_values_r_mut(&mut node.right, callback);
        }
    }
}


//TODO - revoir syntaxe des mut, avant ou après les ':' ?
fn visit_neighbor_values_r<'a, T, C>(link: &'a Link<T>, label: Option<char>, key_tail: &mut Chars, tail_len: usize, range: usize, callback: &mut C)
where C: FnMut (&T) {

    if range == 0 {

        if let Some(label) = label {

            if let Some(value) = get_r(link, label, key_tail) {

                callback(value);
            }
        }

    } else {

        if let Some(ref node) = *link {

            visit_neighbor_values_r(&node.left, label, key_tail, tail_len, range, callback);

            if let Some(ref value) = node.value {

                let new_range = match label {

                    None => range-1,

                    Some(label) => if label==node.label { range } else { range-1 }
                };

                if tail_len <= new_range {

                    callback(value);
                }
            }

            //TODO - Vérifier la libération des objets. Cela arrive-t-il plus rapidement
            //       avec une portée réduite ?
            {
                let new_range = match label {

                    None => range-1,

                    Some(label) => if label==node.label { range } else { range-1 }
                };

                let mut new_tail = key_tail.clone();
                let new_label = new_tail.next();

                let new_len = if tail_len > 0 { tail_len-1 } else { tail_len };

                visit_neighbor_values_r(&node.middle, new_label, &mut new_tail, new_len, new_range, callback);
            }

            visit_neighbor_values_r(&node.right, label, key_tail, tail_len, range, callback);
        }
    }
}


fn visit_neighbor_values_r_mut<'a, T, C>(link: &'a mut Link<T>, label: Option<char>, key_tail: &mut Chars, tail_len: usize, range: usize, callback: &mut C)
where C: FnMut (&mut T) {

    if range == 0 {

        if let Some(label) = label {

            //TODO - Clarifier ça...
            if let Some(/*ref mut*/ value) = get_r_mut(link, label, key_tail) {

                callback(value);
            }
        }

    } else {

        if let Some(ref mut node) = *link {

            let label_tmp = node.label;

            visit_neighbor_values_r_mut(&mut node.left, label, key_tail, tail_len, range, callback);

            if let Some(ref mut value) = node.value {

                let new_range = match label {

                    None => range-1,

                    //TODO - Clarifier ça...
                    Some(label) => if label == /*node.label*/ label_tmp { range } else { range-1 }
                };

                if tail_len <= new_range {

                    callback(value);
                }
            }

            //TODO - Vérifier la libération des objets. Cela arrive-t-il plus rapidement
            //       avec une portée réduite ?
            {
                let new_range = match label {

                    None => range-1,

                    Some(label) => if label == node.label { range } else { range-1 }
                };

                let mut new_tail = key_tail.clone();
                let new_label = new_tail.next();

                let new_len = if tail_len > 0 { tail_len-1 } else { tail_len };

                visit_neighbor_values_r_mut(&mut node.middle, new_label, &mut new_tail, new_len, new_range, callback);
            }

            visit_neighbor_values_r_mut(&mut node.right, label, key_tail, tail_len, range, callback);
        }
    }
}


fn visit_crossword_values_r<'a, T, C>(link: &'a Link<T>, label: char, key_tail: &mut Chars, joker: char, callback: &mut C)
    where C: FnMut (&T) {

    match *link {

        None => return,

        Some(ref node) => {

            if label == joker || label < node.label {

                visit_crossword_values_r(&node.left, label, key_tail, joker, callback);
            }

            if label == joker || label == node.label {

                let mut new_tail = key_tail.clone();
                let new_label = new_tail.next();

                match new_label {

                    None =>  if let Some(ref value) = node.value {

                        callback(value);
                    },

                    Some(label) => visit_crossword_values_r(&node.middle, label, &mut new_tail, joker, callback)
                }
            }

            if label == joker || label > node.label {

                visit_crossword_values_r(&node.right, label, key_tail, joker, callback);
            }
        }
    }
}


fn visit_crossword_values_r_mut<'a, T, C>(link: &'a mut Link<T>, label: char, key_tail: &mut Chars, joker: char, callback: &mut C)
    where C: FnMut (&mut T) {

    match *link {

        None => return,

        Some(ref mut node) => {

            if label == joker || label < node.label {

                visit_crossword_values_r_mut(&mut node.left, label, key_tail, joker, callback);
            }

            if label == joker || label == node.label {

                let mut new_tail = key_tail.clone();
                let new_label = new_tail.next();

                match new_label {

                    None =>  if let Some(ref mut value) = node.value {

                        callback(value);
                    },

                    Some(label) => visit_crossword_values_r_mut(&mut node.middle, label, &mut new_tail, joker, callback)
                }
            }

            if label == joker || label > node.label {

                visit_crossword_values_r_mut(&mut node.right, label, key_tail, joker, callback);
            }
        }
    }
}


fn pretty_print_r<'a, T>(link: &'a Link<T>, writer: &mut Write) {

    match *link {

        None => return,

        Some(ref node) => {

            let value_box = match node.value {

                None => "☐", Some(_) => "☑"
            };

            let _ = writeln!(writer, r#""{:p}" [label=<<TABLE BORDER="0" CELLBORDER="1" CELLSPACING="0"><TR><TD COLSPAN="3">{} {}</TD></TR><TR><TD PORT="l"></TD><TD PORT="m"></TD><TD PORT="r"></TD></TR></TABLE>>]"#, node, value_box, node.label);

            {
                let mut print_edge = |link, start, style| if let &Some(ref child) = link {

                    let _ = writeln!(writer, r#""{:p}":{} -> "{:p}" [style={}]"#, node, start, child, style);
                };

                print_edge(&node.left, "l", "solid");
                print_edge(&node.middle, "m", "bold");
                print_edge(&node.right, "r", "solid");
            }

            pretty_print_r(&node.left, writer);
            pretty_print_r(&node.middle, writer);
            pretty_print_r(&node.right, writer);
        }
    }
}


impl<T> Tst<T> {

    pub fn new() -> Self {

        Tst { root: None, count: 0 }
    }


    // La clé n'est pas consommée (contrairement au treemap)
    pub fn insert(&mut self, key: &str, value: T) -> Option<T> {

        let mut key_tail = key.chars();

        match key_tail.next() {

            None => Some(value),

            Some(label) => {

                let old_value = insert_r(&mut self.root, label, key_tail, value);

                if old_value.is_none() {

                    self.count += 1;
                }

                old_value
            }
        }
    }


    pub fn get(&self, key: &str) -> Option<&T> {

        let mut key_tail = key.chars();

        match key_tail.next() {

            None => None,

            Some(label) => get_r(&self.root, label, &mut key_tail)
        }
    }


    pub fn get_mut(&mut self, key: &str) -> Option<&mut T> {

        let mut key_tail = key.chars();

        match key_tail.next() {

            None => None,

            Some(label) => get_r_mut(&mut self.root, label, &mut key_tail)
        }
    }


    pub fn remove(&mut self, key: &str) -> Option<T> {

        let mut key_tail = key.chars();

        let (prune, old_value) = match key_tail.next() {

            None => (false, None),

            Some(label) => remove_r(&mut self.root, label, &mut key_tail)
        };

        if prune {

            self.root = None;
        }

        if old_value.is_some() {

            self.count -= 1;
        }

        old_value
    }


    pub fn len(&self) -> usize {

        self.count
    }


    pub fn stat(&self) -> Stats {

        let empty_stats: Stats = Default::default();

        let mut stats = stat_r(empty_stats, &self.root, 0, 0, 0);

        stats.bytes.node = mem::size_of::<Node<T>>();
        stats.bytes.total = mem::size_of::<Tst<T>>()+stats.count.nodes*stats.bytes.node;

        stats
    }


    pub fn clear(&mut self) {

        self.root = None;
        self.count = 0;
    }


    pub fn visit_values<C>(&self, mut callback: C)
    where C: FnMut (&T) {

        visit_values_r(&self.root, &mut callback);
    }


    pub fn visit_values_mut<C>(&mut self, mut callback: C)
    where C: FnMut (&mut T) {

        visit_values_r_mut(&mut self.root, &mut callback);
    }


    pub fn visit_complete_values<C>(&self, key: &str, mut callback: C)
    where C: FnMut (&T) {

        let mut key_tail = key.chars();

        match key_tail.next() {

            None => visit_values_r(&self.root, &mut callback),

            Some(label) => {

                let new_root = find_complete_root_r(&self.root, label, key_tail);
                visit_complete_values_r(new_root, &mut callback)
            }
        }
    }


    pub fn visit_complete_values_mut<C>(&mut self, key: &str, mut callback: C)
    where C: FnMut (&mut T) {

        let mut key_tail = key.chars();

        match key_tail.next() {

            None => visit_values_r_mut(&mut self.root, &mut callback),

            Some(label) => {

                let mut new_root = find_complete_root_r_mut(&mut self.root, label, key_tail);
                visit_complete_values_r_mut(&mut new_root, &mut callback)
            }
        }
    }


    pub fn visit_neighbor_values<C>(&self, key: &str, dist: usize, mut callback: C)
    where C: FnMut (&T) {

        let mut key_tail = key.chars();
        let label = key_tail.next();
        let tail_len = if key.len() == 0 { 0 } else { key.len()-1 };

        visit_neighbor_values_r(&self.root, label, &mut key_tail, tail_len, dist, &mut callback);
    }


    pub fn visit_neighbor_values_mut<C>(&mut self, key: &str, dist: usize, mut callback: C)
    where C: FnMut (&mut T) {

        let mut key_tail = key.chars();
        let label = key_tail.next();
        let tail_len = if key.len() == 0 { 0 } else { key.len()-1 };

        visit_neighbor_values_r_mut(&mut self.root, label, &mut key_tail, tail_len, dist, &mut callback);
    }


    pub fn visit_crossword_values<C>(&self, key: &str, joker: char, mut callback: C)
    where C: FnMut (&T) {

        let mut key_tail = key.chars();

        match key_tail.next() {

            None => return,

            Some(label) => visit_crossword_values_r(&self.root, label, &mut key_tail, joker, &mut callback)
        }
    }


    pub fn visit_crossword_values_mut<C>(&mut self, key: &str, joker: char, mut callback: C)
    where C: FnMut (&mut T) {

        let mut key_tail = key.chars();

        match key_tail.next() {

            None => return,

            Some(label) => visit_crossword_values_r_mut(&mut self.root, label, &mut key_tail, joker, &mut callback)
        }
    }


    pub fn pretty_print(&self, writer: &mut Write) {

        let _ = writeln!(writer, "digraph {{");
        let _ = writeln!(writer, "node [shape=plaintext]");

        pretty_print_r(&self.root, writer);

        let _ = writeln!(writer, "}}");

    }


    pub fn iter(&self) -> TstIterator<T> {

        TstIterator::<T>::new(&self)
    }


    pub fn iter_complete(&self, prefix: &str) -> TstCompleteIterator<T> {

        TstCompleteIterator::<T>::new(&self, prefix)
    }


    pub fn iter_neighbor<'a, 'b>(&'a self, key: &'b str, range: usize) -> TstNeighborIterator<'a, 'b, T> {

        TstNeighborIterator::<T>::new(&self, key, range)
    }


    pub fn iter_crossword<'a, 'b>(&'a self, key: &'b str, joker: char) -> TstCrosswordIterator<'a, 'b, T> {

        TstCrosswordIterator::<T>::new(&self, key, joker)
    }
}


#[macro_export]
macro_rules! tst {

    () => {{
        $crate::Tst::new()
    }};

    ($($key:expr => $value:expr,)+) => (tst!($($key => $value),+));

    ($($key: expr => $val: expr),*) => {{

        let mut tst = $crate::Tst::new();
        $(
            tst.insert($key, $val);
        )*

        tst
    }};
}


#[derive(Debug, PartialEq)]
enum TstIteratorAction {

    GoLeft,
    Visit,
    GoMiddle,
    GoRight
}

use self::TstIteratorAction::*;


#[derive(Debug)]
pub struct TstIterator<'a, T: 'a> {

    todo_i: Vec<(&'a Node<T>, TstIteratorAction)>,
    last_i: Option<&'a Node<T>>,

    todo_j: Vec<(&'a Node<T>, TstIteratorAction)>,
    last_j: Option<&'a Node<T>>
}


macro_rules! gen_it_path {

    ($path_of_x:ident, $todo_x:ident, $a1:expr, $a2:expr) => (

        pub fn $path_of_x(&self) -> String {

            let mut path = String::new();

            for todo in self.$todo_x.iter() {

                if todo.1 == $a1 || todo.1 == $a2 {

                    path.push(todo.0.label);
                }
            }

            path
        }
    );
}


impl<'a, T> TstIterator<'a, T> {

    pub fn new(tst: &'a Tst<T>) -> Self {

        TstIterator::new_from_root(&tst.root)
    }


    fn new_from_root(root: &'a Link<T>) -> Self {

        let mut it = TstIterator {

            todo_i: Vec::new(), last_i: None,
            todo_j: Vec::new(), last_j: None,
        };

        if let Some(ref node) = root {

            //TODO - Comprendre exactement comment on se débarasse de la box ici
            //no method named `paf` found for type `&std::boxed::Box<tst::Node<T>>`
            //node.paf();

            it.todo_i.push((node, GoLeft));
            it.todo_j.push((node, GoRight));
        }

        it
    }


    gen_it_path!(current_key, todo_i, GoMiddle, GoRight);
    gen_it_path!(current_key_back, todo_j, Visit, GoLeft);
}


impl<'a, T> Iterator for TstIterator<'a, T> {

    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {

        let mut found = None;

        while let Some((node, action)) = self.todo_i.pop() {

            match action {

                GoLeft => {

                    self.todo_i.push((node, Visit));

                    if let Some(ref child) = node.left {

                        self.todo_i.push((child, GoLeft));
                    }
                }

                Visit => {

                    if node.value.is_some() {

                        if let Some(node_j) = self.last_j {

                            if ptr::eq(node, node_j) {

                                self.todo_i.clear();
                                self.todo_j.clear();

                                found = None;
                                break;
                            }
                        }
                    }

                    self.todo_i.push((node, GoMiddle));

                    if let Some(ref value) = node.value {

                        self.last_i = Some(node);
                        found = Some(value);

                        break;
                    }
                }

                GoMiddle => {

                    self.todo_i.push((node, GoRight));

                    if let Some(ref child) = node.middle {

                        self.todo_i.push((child, GoLeft));
                    }
                }

                GoRight => {

                    if let Some(ref child) = node.right {

                        self.todo_i.push((child, GoLeft));
                    }
                }
            }
        }

        found
    }
}


impl<'a, T> IntoIterator for &'a Tst<T> {

    type Item = &'a T;
    type IntoIter = TstIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {

        self.iter()
    }
}


impl<'a, T> DoubleEndedIterator for TstIterator<'a, T> {

    fn next_back(&mut self) -> Option<&'a T> {

        let mut found = None;

        while let Some((node, action)) = self.todo_j.pop() {

            match action {

                GoRight => {

                    self.todo_j.push((node, GoMiddle));

                    if let Some(ref child) = node.right {

                        self.todo_j.push((child, GoRight));
                    }
                }

                Visit => {

                    if node.value.is_some() {

                        if let Some(node_i) = self.last_i {

                            if ptr::eq(node, node_i) {

                                self.todo_i.clear();
                                self.todo_j.clear();

                                found = None;
                                break;
                            }
                        }
                    }

                    self.todo_j.push((node, GoLeft));

                    if let Some(ref value) = node.value {

                        self.last_j = Some(node);
                        found = Some(value);

                        break;
                    }
                }

                GoMiddle => {

                    self.todo_j.push((node, Visit));

                    if let Some(ref child) = node.middle {

                        self.todo_j.push((child, GoRight));
                    }
                }

                GoLeft => {

                    if let Some(ref child) = node.left {

                        self.todo_j.push((child, GoRight));
                    }
                }
            }
        }

        found
    }
}


#[derive(Debug)]
pub struct TstCompleteIterator<'a, T: 'a> {

    it: TstIterator<'a, T>,
    prefix: String
}


impl<'a, T> TstCompleteIterator<'a, T> {

    //TODO - On consomme uns String ou on prend une &str qui est copiée (cohérence interface) ?
    pub fn new(tst: &'a Tst<T>, key_prefix: &str) -> Self {

        let mut key_tail = key_prefix.chars();

        TstCompleteIterator {

            it : match key_tail.next() {

                None => TstIterator::<T>::new(tst),

                Some(label) => {

                    let new_root = find_complete_root_r(&tst.root, label, key_tail);
                    TstIterator::<T>::new_from_root(new_root)
                }
            },

            prefix: key_prefix.to_string()
        }
    }


    pub fn current_key(&self) -> String {

        self.prefix.clone() + &self.it.current_key()
    }


    pub fn current_key_back(&self) -> String {

        self.prefix.clone() + &self.it.current_key_back()
    }
}


impl<'a, T> Iterator for TstCompleteIterator<'a, T> {

    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {

        self.it.next()
    }
}


impl<'a, T> DoubleEndedIterator for TstCompleteIterator<'a, T> {

    fn next_back(&mut self) -> Option<&'a T> {

       self.it.next_back()
    }
}


#[derive(Debug)]
pub struct TstNeighborIterator<'a, 'b, T: 'a> {

    todo_i: Vec<(&'a Node<T>, TstIteratorAction, Option<char>, Chars<'b>, usize, usize)>,
    last_i: Option<&'a Node<T>>,

    todo_j: Vec<(&'a Node<T>, TstIteratorAction, Option<char>, Chars<'b>, usize, usize)>,
    last_j: Option<&'a Node<T>>
}


impl<'a, 'b, T> TstNeighborIterator<'a, 'b, T> {

    pub fn new(tst: &'a Tst<T>, key: &'b str, range: usize) -> Self {

        let mut it = TstNeighborIterator {

            todo_i: Vec::new(), last_i: None,
            todo_j: Vec::new(), last_j: None,
        };

        if let Some(ref node) = &tst.root {

            let mut key_tail = key.chars();
            let label = key_tail.next();
            let tail_len = if key.len() == 0 { 0 } else { key.len()-1 };

            it.todo_i.push((node, GoLeft, label, key_tail.clone(), tail_len, range));
            it.todo_j.push((node, GoRight, label, key_tail, tail_len, range));
        }

        it
    }


    gen_it_path!(current_key, todo_i, GoMiddle, GoRight);
    gen_it_path!(current_key_back, todo_j, Visit, GoLeft);
}


impl<'a, 'b, T> Iterator for TstNeighborIterator<'a, 'b, T> {

    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {

        let mut found = None;

        while let Some((node, action, label, mut key_tail, tail_len, range)) = self.todo_i.pop() {

            match action {

                GoLeft => {

                    self.todo_i.push((node, Visit, label, key_tail.clone(), tail_len, range));

                    if let Some(label) = label {

                        if range == 0 && label >= node.label {

                            continue;
                        }
                    }

                    if let Some(ref child) = node.left {

                        self.todo_i.push((child, GoLeft, label, key_tail, tail_len, range));
                    }
                }

                Visit => {

                    if node.value.is_some() {

                        if let Some(node_j) = self.last_j {

                            if ptr::eq(node, node_j) {

                                self.todo_i.clear();
                                self.todo_j.clear();

                                found = None;
                                break;
                            }
                        }
                    }

                    self.todo_i.push((node, GoMiddle, label, key_tail, tail_len, range));

                    if let Some(ref value) = node.value {

                        let delta = match label {

                            None => 1,

                            Some(label) => if label==node.label { 0 } else { 1 }

                        };

                        if range >= delta {

                            let new_range = range - delta;

                            if tail_len  <= new_range {

                                self.last_i = Some(node);
                                found = Some(value);

                                break;
                            }
                        }
                    }
                }

                GoMiddle => {

                    self.todo_i.push((node, GoRight, label, key_tail.clone(), tail_len, range));

                    let delta = match label {

                        None => 1,

                        Some(label) => if label==node.label { 0 } else { 1 }
                    };

                    if range >= delta {

                        let new_range = range - delta;

                        let new_label = key_tail.next();
                        let new_len = if tail_len > 0 { tail_len-1 } else { tail_len };

                        if let Some(ref child) = node.middle {

                            self.todo_i.push((child, GoLeft, new_label, key_tail, new_len, new_range));
                        }
                    }
                }

                GoRight => {

                    if let Some(label) = label {

                        if range == 0 && label <= node.label {

                            continue;
                        }
                    }

                    if let Some(ref child) = node.right {

                        self.todo_i.push((child, GoLeft, label, key_tail, tail_len, range));
                    }
                }
            }
        }

        found
    }
}


impl<'a, 'b, T> DoubleEndedIterator for TstNeighborIterator<'a, 'b, T> {

    fn next_back(&mut self) -> Option<&'a T> {

        let mut found = None;

        while let Some((node, action, label, mut key_tail, tail_len, range)) = self.todo_j.pop() {

            match action {

                GoRight => {

                    self.todo_j.push((node, GoMiddle, label, key_tail.clone(), tail_len, range));

                    if let Some(label) = label {

                        if range == 0 && label <= node.label {

                            continue;
                        }
                    }

                    if let Some(ref child) = node.right {

                        self.todo_j.push((child, GoRight, label, key_tail, tail_len, range));
                    }
                }

                Visit => {

                    if node.value.is_some() {

                        if let Some(node_i) = self.last_i {

                            if ptr::eq(node, node_i) {

                                self.todo_i.clear();
                                self.todo_j.clear();

                                found = None;
                                break;
                            }
                        }
                    }

                    self.todo_j.push((node, GoLeft, label, key_tail, tail_len, range));

                    if let Some(ref value) = node.value {

                        let delta = match label {

                            None => 1,

                            Some(label) => if label==node.label { 0 } else { 1 }

                        };

                        if range >= delta {

                            let new_range = range - delta;

                            if tail_len  <= new_range {

                                self.last_j = Some(node);
                                found = Some(value);

                                break;
                            }
                        }
                    }
                }

                GoMiddle => {

                    self.todo_j.push((node, Visit, label, key_tail.clone(), tail_len, range));

                    let delta = match label {

                        None => 1,

                        Some(label) => if label==node.label { 0 } else { 1 }

                    };

                    if range >= delta {

                        let new_range = range - delta;

                        let new_label = key_tail.next();
                        let new_len = if tail_len > 0 { tail_len-1 } else { tail_len };

                        if let Some(ref child) = node.middle {

                            self.todo_j.push((child, GoRight, new_label, key_tail, new_len, new_range));
                        }
                    }
                }

                GoLeft => {

                    if let Some(label) = label {

                        if range == 0 && label >= node.label {

                            continue;
                        }
                    }

                    if let Some(ref child) = node.left {

                        self.todo_j.push((child, GoRight, label, key_tail, tail_len, range));
                    }
                }
            }
        }

        found
    }
}


#[derive(Debug)]
pub struct TstCrosswordIterator<'a, 'b, T: 'a> {

    todo_i: Vec<(&'a Node<T>, TstIteratorAction, char, Chars<'b>, usize)>,
    last_i: Option<&'a Node<T>>,

    todo_j: Vec<(&'a Node<T>, TstIteratorAction, char, Chars<'b>, usize)>,
    last_j: Option<&'a Node<T>>,

    joker: char
}


impl<'a, 'b, T> TstCrosswordIterator<'a, 'b, T> {

    pub fn new(tst: &'a Tst<T>, key: &'b str, joker: char) -> Self {

        let mut it = TstCrosswordIterator {

            todo_i: Vec::new(), last_i: None,
            todo_j: Vec::new(), last_j: None,
            joker: joker,

        };

        if let Some(ref node) = &tst.root {

            let mut key_tail = key.chars();

            if let Some(label) = key_tail.next() {

                let tail_len = key.len()-1;

                it.todo_i.push((node, GoLeft, label, key_tail.clone(), tail_len));
                it.todo_j.push((node, GoRight, label, key_tail, tail_len));
            }
        }

        it
    }


    gen_it_path!(current_key, todo_i, GoMiddle, GoRight);
    gen_it_path!(current_key_back, todo_j, Visit, GoLeft);
}


impl<'a, 'b, T> Iterator for TstCrosswordIterator<'a, 'b, T> {

    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {

        let mut found = None;

        while let Some((node, action, label, mut key_tail, tail_len)) = self.todo_i.pop() {

            match action {

                GoLeft => {

                    self.todo_i.push((node, Visit, label, key_tail.clone(), tail_len));

                    if label == self.joker || label < node.label {

                        if let Some(ref child) = node.left {

                            self.todo_i.push((child, GoLeft, label, key_tail, tail_len));
                        }
                    }
                }

                Visit => {

                    if node.value.is_some() {

                        if let Some(node_j) = self.last_j {

                            if ptr::eq(node, node_j) {

                                self.todo_i.clear();
                                self.todo_j.clear();

                                found = None;
                                break;
                            }
                        }
                    }

                    self.todo_i.push((node, GoMiddle, label, key_tail, tail_len));

                    if let Some(ref value) = node.value {

                        if tail_len == 0 && (label == self.joker || label == node.label) {

                            self.last_i = Some(node);
                            found = Some(value);

                            break;
                        }
                    }
                }

                GoMiddle => {

                    self.todo_i.push((node, GoRight, label, key_tail.clone(), tail_len));

                    if label == self.joker || label == node.label {

                        if let Some(ref child) = node.middle {

                            if let Some(new_label) = key_tail.next() {

                                self.todo_i.push((child, GoLeft, new_label, key_tail, tail_len-1));
                            }
                        }
                    }
                }

                GoRight => {

                    if label == self.joker || label > node.label {

                        if let Some(ref child) = node.right {

                            self.todo_i.push((child, GoLeft, label, key_tail, tail_len));
                        }
                    }
                }
            }
        }

        found
    }
}


impl<'a, 'b, T> DoubleEndedIterator for TstCrosswordIterator<'a, 'b, T> {

    fn next_back(&mut self) -> Option<&'a T> {

        let mut found = None;

        while let Some((node, action, label, mut key_tail, tail_len)) = self.todo_j.pop() {

            match action {

                GoRight => {

                    self.todo_j.push((node, GoMiddle, label, key_tail.clone(), tail_len));

                    if label == self.joker || label > node.label {

                        if let Some(ref child) = node.right {

                            self.todo_j.push((child, GoRight, label, key_tail, tail_len));
                        }
                    }
                }

                Visit => {

                    if node.value.is_some() {

                        if let Some(node_i) = self.last_i {

                            if ptr::eq(node, node_i) {

                                self.todo_i.clear();
                                self.todo_j.clear();

                                found = None;
                                break;
                            }
                        }
                    }

                    self.todo_j.push((node, GoLeft, label, key_tail, tail_len));

                    if let Some(ref value) = node.value {

                        if tail_len == 0 && (label == self.joker || label == node.label) {

                            self.last_j = Some(node);
                            found = Some(value);

                            break;
                        }
                    }
                }

                GoMiddle => {

                    self.todo_j.push((node, Visit, label, key_tail.clone(), tail_len));

                    if label == self.joker || label == node.label {

                        if let Some(ref child) = node.middle {

                            if let Some(new_label) = key_tail.next() {

                                self.todo_j.push((child, GoRight, new_label, key_tail, tail_len-1));
                            }
                        }
                    }
                }

                GoLeft => {

                    if label == self.joker || label < node.label {

                        if let Some(ref child) = node.left {

                            self.todo_j.push((child, GoRight, label, key_tail, tail_len));
                        }
                    }
                }
            }
        }

        found
    }
}
