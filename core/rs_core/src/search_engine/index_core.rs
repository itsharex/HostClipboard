use std::collections::HashMap;
use log::debug;
use crate::db::entities::host_clipboard::Model;

#[derive(Debug)]
pub struct Trie {
    pub root: TrieNode,
}

#[derive(Debug)]
struct TrieNode {
    children: HashMap<char, Box<TrieNode>>,
    doc_ids: Vec<(i32, i32, i64)>, // (doc_id, doc_type, timestamp)
}
impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            doc_ids: Vec::new(),
        }
    }
}

impl Trie {
    pub fn new() -> Self {
        Trie {
            root: TrieNode::new(),
        }
    }
    pub fn insert(&mut self, doc: Model) {
        for (i, _) in doc.content.char_indices() {
            let mut node = &mut self.root;
            for ch in doc.content[i..].chars() {
                node = node.children.entry(ch).or_insert_with(|| Box::new(TrieNode::new()));
                node.doc_ids.push((doc.id, doc.r#type, doc.timestamp));
            }
        }
    }

    pub fn search(&self, query: &str, n: usize, doc_type: Option<i32>) -> Vec<i32> {
        // debug!("search.res{:?}", self.root);
        let mut node = &self.root;
        for ch in query.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        let mut res = node.doc_ids.clone();
        // 如果提供了 doc_type，先进行筛选
        if let Some(dtype) = doc_type {
            res.retain(|&(_, doc_type, _)| doc_type == dtype);
        }

        // 按照 timestamp 排序
        res.sort_by(|a, b| b.2.cmp(&a.2));
        // 取前 n 个
        res.truncate(n);
        res.into_iter().map(|(first, _, _)| first).collect()
    }
}
