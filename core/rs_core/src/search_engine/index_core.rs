use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashMap;

use crate::db::entities::host_clipboard::Model;

pub struct SearchEngine {
    pub documents: HashMap<i32, Model>,
    trie_root: TrieNode,
}

struct TrieNode {
    children: HashMap<char, Box<TrieNode>>,
    doc_ids: Vec<(i32, i32)>, // (doc_id, doc_type)
}
impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            doc_ids: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct InvertedIndexEntry {
    doc_id: i32,
    term_frequency: u32,
    doc_type: i32,
}

#[derive(PartialEq, Eq)]
struct SearchResult {
    doc_id: i32,
    score: i64,
    timestamp: i64,
}
impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .cmp(&other.score)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
            .then_with(|| self.doc_id.cmp(&other.doc_id))
    }
}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        SearchEngine {
            documents: HashMap::new(),
            trie_root: TrieNode::new(),
        }
    }
    pub fn add_document(&mut self, doc: Model) {
        let doc_id = doc.id;
        let doc_type = doc.r#type;

        for ch in doc.content.chars() {
            let mut current_node = &mut self.trie_root;
            current_node = current_node
                .children
                .entry(ch)
                .or_insert_with(|| Box::new(TrieNode::new()));
            if !current_node.doc_ids.contains(&(doc_id, doc_type)) {
                current_node.doc_ids.push((doc_id, doc_type));
            }
        }

        self.documents.insert(doc_id, doc);
    }

    pub fn search(&self, query: &str, n: usize, doc_type: Option<i32>) -> Vec<i32> {
        let mut scores: HashMap<i32, usize> = HashMap::new();

        // 对查询字符串中的每个字符开始一次搜索
        for start_char in query.chars() {
            let mut current_node = &self.trie_root;
            let mut matched_length = 0;

            // 从当前字符开始，尝试匹配尽可能长的序列
            for query_char in query[matched_length..].chars() {
                match current_node.children.get(&query_char) {
                    Some(node) => {
                        current_node = node;
                        matched_length += 1;

                        // 更新匹配文档的分数
                        for &(doc_id, entry_doc_type) in &current_node.doc_ids {
                            if doc_type.is_none() || doc_type == Some(entry_doc_type) {
                                *scores.entry(doc_id).or_insert(0) += matched_length;
                            }
                        }
                    }
                    None => break,
                }
            }

            // 如果没有匹配到任何字符，继续下一个起始字符
            if matched_length == 0 {
                continue;
            }
        }

        // 剩余的代码保持不变
        let mut heap = BinaryHeap::with_capacity(n);

        for (doc_id, score) in scores {
            let timestamp = self.documents.get(&doc_id).map_or(0, |doc| doc.timestamp);
            let result = Reverse(SearchResult {
                doc_id,
                score: score as i64,
                timestamp,
            });

            if heap.len() < n {
                heap.push(result);
            } else if result < *heap.peek().unwrap() {
                heap.pop();
                heap.push(result);
            }
        }

        heap.into_sorted_vec()
            .into_iter()
            .map(|Reverse(r)| r.doc_id)
            .collect()
    }
    pub fn remove_document(&mut self, doc_id: i32) {
        if let Some(doc) = self.documents.remove(&doc_id) {
            for ch in doc.content.chars() {
                self.remove_char_for_doc(ch, doc_id, doc.r#type);
            }
        }
    }

    fn remove_char_for_doc(&mut self, ch: char, doc_id: i32, doc_type: i32) {
        let mut current_node = &mut self.trie_root;
        if let Some(node) = current_node.children.get_mut(&ch) {
            node.doc_ids
                .retain(|&(id, t)| id != doc_id || t != doc_type);

            if node.doc_ids.is_empty() && node.children.is_empty() {
                current_node.children.remove(&ch);
            }
        }
    }
}
