use crate::db::entities::host_clipboard::Model;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug)]
pub struct Trie {
    pub timestamp_ids: BTreeMap<i64, HashSet<i32>>,
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
            timestamp_ids: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, doc: &Model) {
        self.td_insert(doc.timestamp, doc.id);

        for (i, _) in doc.content.char_indices() {
            let mut node = &mut self.root;
            for ch in doc.content[i..].chars().flat_map(char::to_lowercase) {
                node = node
                    .children
                    .entry(ch)
                    .or_insert_with(|| Box::new(TrieNode::new()));
                node.doc_ids.push((doc.id, doc.r#type, doc.timestamp));
            }
        }
    }
    pub fn insert_list(&mut self, doc_list: &[Model]) {
        const BATCH_SIZE: usize = 1000;

        // Batch insert timestamps and ids
        for doc in doc_list {
            self.td_insert(doc.timestamp, doc.id)
        }

        // Process documents in batches
        for chunk in doc_list.chunks(BATCH_SIZE) {
            let mut sorted_docs: Vec<_> = chunk.iter().collect();
            sorted_docs.sort_unstable_by(|a, b| a.content.cmp(&b.content));

            for doc in sorted_docs {
                self.insert_doc(doc);
            }
        }
    }


    fn insert_doc(&mut self, doc: &Model) {
        // 对文档的每个起始位置都创建一个完整的路径
        for (i, _) in doc.content.char_indices() {
            let mut node = &mut self.root;
            for ch in doc.content[i..].chars().flat_map(char::to_lowercase) {
                node = node
                    .children
                    .entry(ch)
                    .or_insert_with(|| Box::new(TrieNode::new()));
                node.doc_ids.push((doc.id, doc.r#type, doc.timestamp));
            }
        }
    }

    pub fn search(&self, query: &str, n: u64, type_list: Option<Vec<i32>>) -> Vec<i32> {
        self.search_node(query)
            .map(|node| {
                self.filter_and_sort_results(node, n, |dtype| {
                    type_list
                        .as_ref()
                        .map_or(true, |types| types.contains(&dtype))
                })
            })
            .unwrap_or_default()
    }

    fn search_node(&self, query: &str) -> Option<&TrieNode> {
        let mut node = &self.root;
        for ch in query.chars().flat_map(|c| c.to_lowercase()) {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return None,
            }
        }
        Some(node)
    }

    fn filter_and_sort_results(
        &self,
        node: &TrieNode,
        n: u64,
        type_filter: impl Fn(i32) -> bool,
    ) -> Vec<i32> {
        let mut res: Vec<_> = node
            .doc_ids
            .iter()
            .filter(|&&(_, dtype, _)| type_filter(dtype))
            .map(|&(id, _, timestamp)| (id, timestamp))
            .collect();

        res.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        res.dedup_by(|a, b| a.0 == b.0); // 添加这行来去重
        res.truncate(n as usize);
        res.into_iter().map(|(id, _)| id).collect()
    }

    pub fn delete(&mut self, doc: &Model) {
        delete_helper(&mut self.root, doc, 0);

        if let Some(ids) = self.timestamp_ids.get_mut(&doc.timestamp) {
            ids.remove(&doc.id);
            if ids.is_empty() {
                self.timestamp_ids.remove(&doc.timestamp);
            }
        }
    }

    fn td_insert(&mut self, timestamp: i64, doc_id: i32) {
        self.timestamp_ids
            .entry(timestamp)
            .or_insert_with(HashSet::new)
            .insert(doc_id);
    }

    // 小于 timestamp
    pub fn td_lt_ids(&self, timestamp: i64) -> HashSet<i32> {
        self.timestamp_ids
            .range(..timestamp)
            .flat_map(|(_timestamp, ids)| ids)
            .copied()
            .collect()
    }

    // 大于 timestamp
    pub fn td_gt_ids(&self, timestamp: i64) -> HashSet<i32> {
        self.timestamp_ids
            .range(timestamp..)
            .flat_map(|(_timestamp, ids)| ids)
            .copied()
            .collect()
    }
}
fn delete_helper(node: &mut TrieNode, doc: &Model, index: usize) {
    node.doc_ids.retain(|&(id, _, _)| id != doc.id);

    if index >= doc.content.len() {
        return;
    }

    for (i, ch) in doc.content[index..].char_indices() {
        for lc in ch.to_lowercase() {
            if let Some(child) = node.children.get_mut(&lc) {
                delete_helper(child, doc, index + i + ch.len_utf8());
            }
        }
    }

    node.children
        .retain(|_, child| !child.doc_ids.is_empty() || !child.children.is_empty());
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::entities::host_clipboard::Model;

    fn create_test_doc(id: i32, content: &str, doc_type: i32, timestamp: i64) -> Model {
        Model {
            id,
            content: content.to_string(),
            r#type: doc_type,
            timestamp,
            path: "".to_string(),
            uuid: "".to_string(),
        }
    }

    fn get_test_docs() -> (HashMap<i32, Model>, Vec<Model>) {
        let docs = vec![
            create_test_doc(1, "apple", 1, 1),
            create_test_doc(2, "application", 1, 4),
            create_test_doc(3, "apply", 2, 2),
            create_test_doc(4, "appoint", 2, 6),
            create_test_doc(5, "appointment", 3, 7),
            create_test_doc(6, "苹果商店吃苹果", 3, 8),
            create_test_doc(7, "苹果公司", 1, 9),
            create_test_doc(8, "应用", 2, 11),
            create_test_doc(9, "应用程序", 3, 22),
            create_test_doc(10, "应用商店", 1, 33),
        ];
        (
            docs.clone().into_iter().map(|doc| (doc.id, doc)).collect(),
            docs,
        )
    }

    #[test]
    fn test_insert() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        // 验证插入是否成功
        assert!(!trie.root.children.is_empty());
        assert_eq!(trie.root.children[&'a'].doc_ids.len(), 6); // apple, application(2), apply, appoint, appointment
        assert_eq!(trie.root.children[&'l'].doc_ids.len(), 3); // apple, application, apply
        assert_eq!(trie.root.children[&'店'].doc_ids.len(), 2); // apple, application, apply
    }

    #[test]
    fn test_search() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        // 测试精确匹配
        let results = trie.search("apple", 5, None);
        assert_eq!(results, vec![1]);

        // 测试前缀匹配, 按照 timestamp 排序
        let results = trie.search("app", 5, None);
        let mut app_vec = docs_vals[0..5].to_vec();
        app_vec.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let app_ids: Vec<i32> = app_vec.iter().map(|doc| doc.id).collect();
        assert_eq!(results.len(), 5);
        assert_eq!(results, app_ids);

        // 测试中文匹配
        let results = trie.search("苹果公司", 5, None);
        assert_eq!(results, vec![7]);

        // 测试带有 doc_type 的搜索
        let results = trie.search("应用", 5, Some(vec![3]));
        assert_eq!(results, vec![9]);

        // 测试限制结果数量
        let results = trie.search("app", 3, None);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_delete() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        // 测试删除
        let doc = docs_map.get(&1).unwrap(); // apple
        trie.delete(doc);

        let result = trie.search("apple", 5, None);
        assert_eq!(result.len(), 0);

        let result = trie.search("app", 5, None);
        assert_eq!(result, vec![5, 4, 2, 3]);

        let result = trie.search("l", 5, None);
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_timestamp_ids() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }
        assert_eq!(trie.timestamp_ids.len(), 10);
    }

    #[test]
    fn test_timestamp_ids_lt() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        let mut result: Vec<i32> = trie.td_lt_ids(6).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![1, 2, 3]);

        // 删除 apple
        let doc = docs_map.get(&1).unwrap();
        trie.delete(doc);

        let mut result: Vec<i32> = trie.td_lt_ids(6).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_timestamp_ids_gt() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        let mut result: Vec<i32> = trie.td_gt_ids(9).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![7, 8, 9, 10]);

        // 删除 8:  应用
        let doc = docs_map.get(&8).unwrap();
        trie.delete(doc);

        let mut result: Vec<i32> = trie.td_gt_ids(10).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![9, 10]);
    }

    #[test]
    fn test_case_insensitive_search() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        // 测试大写搜索
        let results = trie.search("APPLE", 5, None);
        assert_eq!(results, vec![1]);

        // 测试混合大小写搜索
        let results = trie.search("ApPlIcAtIoN", 5, None);
        assert_eq!(results, vec![2]);

        // 测试前缀匹配，大写
        let results = trie.search("APP", 5, None);
        let mut app_vec = docs_vals[0..5].to_vec();
        app_vec.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let app_ids: Vec<i32> = app_vec.iter().map(|doc| doc.id).collect();
        assert_eq!(results, app_ids);

        // 测试中文不受影响
        let results = trie.search("苹果公司", 5, None);
        assert_eq!(results, vec![7]);

        // 测试带有 doc_type 的大小写不敏感搜索
        let results = trie.search("APPly", 5, Some(vec![2]));
        assert_eq!(results, vec![3]);
    }

    #[test]
    fn test_case_insensitive_delete() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        // 测试删除（使用大写）
        let mut upper_case_doc = docs_map.get(&1).unwrap().clone(); // apple
        upper_case_doc.content = upper_case_doc.content.to_uppercase();
        trie.delete(&upper_case_doc);

        let result = trie.search("apple", 5, None);
        assert_eq!(result.len(), 0);

        let result = trie.search("APP", 5, None);
        assert_eq!(result, vec![5, 4, 2, 3]);
    }

    #[test]
    fn test_search_type_list() {
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_test_docs();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        // 测试单一类型
        let results = trie.search("app", 5, Some(vec![1]));
        assert_eq!(results, vec![2, 1]);

        // 测试多个类型
        let results = trie.search("app", 5, Some(vec![1, 2]));
        assert_eq!(results, vec![4, 2, 3, 1]);

        // 测试所有类型（不提供类型列表）
        let results = trie.search("app", 5, None);
        assert_eq!(results, vec![5, 4, 2, 3, 1]);

        // 测试空类型列表
        let results: Vec<i32> = trie.search("app", 5, Some(vec![]));
        assert_eq!(results, Vec::<i32>::new());

        // 测试不存在的类型
        let results: Vec<i32> = trie.search("app", 5, Some(vec![4]));
        assert_eq!(results, Vec::<i32>::new());

        // 测试限制结果数量
        let results = trie.search("app", 2, Some(vec![1, 2]));
        assert_eq!(results, vec![4, 2]);

        // 测试中文搜索
        let results = trie.search("应用", 5, Some(vec![1, 2, 3]));
        assert_eq!(results, vec![10, 9, 8]);

        // 测试大小写不敏感
        let results = trie.search("APP", 5, Some(vec![1, 2]));
        assert_eq!(results, vec![4, 2, 3, 1]);
    }

    #[test]
    fn test_bad_case() {
        fn get_bad_case() -> (HashMap<i32, Model>, Vec<Model>) {
            let docs = vec![
                create_test_doc(1, "[2024-07-20 09:47:56.778839 +08:00] INFO [/Users/zeke/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/sea-orm-migration-0.12.15/src/migrator.rs:374] No pending migrations", 0, 2),
                create_test_doc(2, "/Users/zeke/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/sea-orm-migration-0.12.15/src/migrator.rs:369", 1, 4),
                create_test_doc(3, "initializeClipboardHelper", 1, 7),
                create_test_doc(4, "work", 1, 8),
                create_test_doc(5, "workspace", 1, 8),
            ];
            (
                docs.clone().into_iter().map(|doc| (doc.id, doc)).collect(),
                docs,
            )
        }
        let mut trie = Trie::new();
        let (docs_map, docs_vals) = get_bad_case();
        for doc in &docs_vals {
            trie.insert(doc);
        }

        let result = trie.search("i", 4, None);
        assert_eq!(result, vec![3, 2, 1])
    }
}
