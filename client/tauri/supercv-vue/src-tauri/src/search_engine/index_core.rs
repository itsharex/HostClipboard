use crate::db::entities::host_clipboard::Model;
use rayon::prelude::*;
use std::collections::Bound::{Excluded, Unbounded};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub struct Trie {
    pub timestamp_ids: Arc<Mutex<BTreeMap<i64, HashSet<i32>>>>,
    pub root: Arc<Mutex<TrieNode>>,
}

#[derive(Debug, Clone)]
struct TrieNode {
    children: HashMap<char, Arc<Mutex<TrieNode>>>,
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
            root: Arc::new(Mutex::new(TrieNode::new())),
            timestamp_ids: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn insert_list(&self, docs: &[Model]) {
        docs.par_iter().for_each(|doc| {
            {
                let mut timestamp_ids = self.timestamp_ids.lock().unwrap();
                timestamp_ids
                    .entry(doc.timestamp)
                    .or_insert_with(HashSet::new)
                    .insert(doc.id);
            }

            let lowercase_content = doc.content.to_lowercase();
            for (i, _) in lowercase_content.char_indices() {
                let mut current = self.root.clone();
                for ch in lowercase_content[i..].chars() {
                    let next = {
                        let mut node = current.lock().unwrap();
                        node.children
                            .entry(ch)
                            .or_insert_with(|| Arc::new(Mutex::new(TrieNode::new())))
                            .clone()
                    };
                    current = next;
                    let mut node = current.lock().unwrap();
                    if !node.doc_ids.iter().any(|&(id, _, _)| id == doc.id) {
                        node.doc_ids.push((doc.id, doc.r#type, doc.timestamp));
                    }
                }
            }
        });
    }

    pub fn search(&self, query: &str, n: u64, type_list: Option<Vec<i32>>) -> Vec<i32> {
        self.search_node(query)
            .map(|node|  // 如果搜索到了对应的节点
        {
            self.filter_and_sort_results(node, n, |dtype| {  // 过滤和排序结果
                type_list
                    .as_ref()  // 如果type_list是Some，获取其引用
                    .map_or(true, |types| types.contains(&dtype))  // 如果类型列表不为空，检查dtype是否在列表中
            })
        })
            .unwrap_or_default() // 如果搜索节点失败（即返回None），则返回默认的空向量
    }

    fn search_node(&self, query: &str) -> Option<Arc<Mutex<TrieNode>>> {
        let mut current = self.root.clone(); // 从根节点开始搜索
        for ch in query.chars().flat_map(|c| c.to_lowercase()) {
            // 将查询字符串转换为小写字符序列
            let next = {
                let node = current.lock().unwrap(); // 获取当前节点的锁
                node.children.get(&ch).cloned() // 尝试在当前节点的子节点中找到当前字符对应的节点
            };
            match next {
                Some(child) => current = child, // 如果找到了，将当前节点更新为子节点
                None => return None,            // 如果没有找到，返回None表示搜索失败
            }
        }
        Some(current) // 返回匹配查询字符串的最后一个节点
    }

    fn filter_and_sort_results(
        &self,
        node: Arc<Mutex<TrieNode>>,
        n: u64,
        type_filter: impl Fn(i32) -> bool,
    ) -> Vec<i32> {
        let mut res = Vec::new(); // 用于存储结果的向量
        let mut stack = vec![node]; // 使用栈来遍历Trie树的节点

        while let Some(current) = stack.pop() {
            // 循环直到栈为空
            let current = current.lock().unwrap(); // 获取当前节点的锁
            res.extend(
                current
                    .doc_ids
                    .iter() // 遍历当前节点的文档ID
                    .filter(|&&(_, dtype, _)| type_filter(dtype)) // 过滤文档类型
                    .map(|&(id, _, timestamp)| (id, timestamp)), // 提取文档ID和时间戳
            );

            for child in current.children.values() {
                // 将当前节点的所有子节点加入栈中
                stack.push(child.clone());
            }
        }

        res.sort_unstable_by(|a, b| b.1.cmp(&a.1)); // 按时间戳降序排序
        res.dedup_by(|a, b| a.0 == b.0); // 删除重复的文档ID
        res.truncate(n as usize); // 截断结果到指定的数量n
        res.into_iter().map(|(id, _)| id).collect() // 返回文档ID列表
    }

    pub fn delete(&mut self, doc: &Model) {
        let mut root = self.root.lock().unwrap();
        let lowercase_content = doc.content.to_lowercase();
        let char_count = lowercase_content.chars().count();

        // 删除所有子字符串
        for i in 0..char_count {
            delete_helper(&mut root, doc.id, &lowercase_content, i);
        }

        let mut timestamp_ids = self.timestamp_ids.lock().unwrap();
        if let Some(ids) = timestamp_ids.get_mut(&doc.timestamp) {
            ids.remove(&doc.id);
            if ids.is_empty() {
                timestamp_ids.remove(&doc.timestamp);
            }
        }
    }

    fn td_insert(&self, timestamp: i64, doc_id: i32) {
        let mut timestamp_ids = self.timestamp_ids.lock().unwrap();
        timestamp_ids
            .entry(timestamp)
            .or_insert_with(HashSet::new)
            .insert(doc_id);
    }

    pub fn td_lt_ids(&self, timestamp: i64) -> HashSet<i32> {
        let timestamp_ids = self.timestamp_ids.lock().unwrap();
        timestamp_ids
            .range(..timestamp)
            .flat_map(|(_timestamp, ids)| ids)
            .copied()
            .collect()
    }

    pub fn td_gt_ids(&self, timestamp: i64) -> HashSet<i32> {
        let timestamp_ids = self.timestamp_ids.lock().unwrap();
        timestamp_ids
            .range((Excluded(&timestamp), Unbounded))
            .flat_map(|(_timestamp, ids)| ids)
            .copied()
            .collect()
    }
}
fn delete_helper(node: &mut TrieNode, doc_id: i32, content: &str, index: usize) -> bool {
    // 删除当前节点中的文档ID
    node.doc_ids.retain(|&(id, _, _)| id != doc_id);

    // 如果已经到达内容末尾，检查是否需要删除当前节点
    if index >= content.chars().count() {
        return node.doc_ids.is_empty() && node.children.is_empty();
    }

    // 获取当前字符
    let current_char = content.chars().nth(index).unwrap();

    // 递归删除子节点
    let mut should_remove_child = false;
    if let Some(child) = node.children.get_mut(&current_char) {
        let mut child = child.lock().unwrap();
        should_remove_child = delete_helper(&mut child, doc_id, content, index + 1);
    }

    // 如果子节点应该被删除，则从children中移除
    if should_remove_child {
        node.children.remove(&current_char);
    }

    // 如果当前节点的doc_ids为空且没有子节点，则应该被删除
    node.doc_ids.is_empty() && node.children.is_empty()
}
fn print_trie_state(trie: &Arc<Mutex<Trie>>) {
    let binding = trie.lock().unwrap();
    let root = binding.root.lock().unwrap();
    print_node(&root, 0);
}

fn print_node(node: &TrieNode, depth: usize) {
    println!("{}Node: {:?}", "  ".repeat(depth), node.doc_ids);
    for (ch, child) in &node.children {
        println!("{}Child '{}': ", "  ".repeat(depth), ch);
        print_node(&child.lock().unwrap(), depth + 1);
    }
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
        let trie = Trie::new();
        let (_, docs_vals) = get_test_docs();
        trie.insert_list(&docs_vals);

        // 验证插入是否成功
        let root = trie.root.lock().unwrap();
        assert!(!root.children.is_empty());

        // 检查 'a' 开头的单词
        if let Some(node) = root.children.get(&'a') {
            let node = node.lock().unwrap();
            assert_eq!(node.doc_ids.len(), 5); // apple, application, apply, appoint, appointment
        } else {
            panic!("'a' node not found");
        }

        if let Some(node) = root.children.get(&'p') {
            let node = node.lock().unwrap();
            assert_eq!(node.doc_ids.len(), 5); // apple, application, apply, appoint, appointment
        } else {
            panic!("'p' node not found");
        }

        if let Some(node) = root.children.get(&'l') {
            let node = node.lock().unwrap();
            assert_eq!(node.doc_ids.len(), 3); // apple, application, apply, appoint, appointment
        } else {
            panic!("'l' node not found");
        }

        // 检查 '店' 的情况
        if let Some(shop_node) = root.children.get(&'店') {
            let shop_node = shop_node.lock().unwrap();
            assert_eq!(shop_node.doc_ids.len(), 2); // 苹果商店吃苹果, 应用商店
        } else {
            panic!("'店' node not found");
        }
    }

    #[test]
    fn test_search() {
        let trie = Trie::new();
        let (_, docs_vals) = get_test_docs();
        trie.insert_list(&docs_vals);

        // 测试精确匹配
        let results = trie.search("apple", 5, None);
        assert_eq!(results, vec![1]);

        // 测试前缀匹配, 按照 timestamp 排序
        let results = trie.search("app", 5, None);
        assert_eq!(results, vec![5, 4, 2, 3, 1]);

        // 测试中文匹配
        let results = trie.search("苹果公司", 5, None);
        assert_eq!(results, vec![7]);

        // 测试带有 doc_type 的搜索
        let results = trie.search("应用", 5, Some(vec![3]));
        assert_eq!(results, vec![9]);

        // 测试限制结果数量
        let results = trie.search("app", 3, None);
        assert_eq!(results, vec![5, 4, 2]);
    }

    #[test]
    fn test_delete() {
        let trie = Arc::new(Mutex::new(Trie::new()));
        let (docs_map, docs_vals) = get_test_docs();
        trie.lock().unwrap().insert_list(&docs_vals);

        // 打印初始状态
        // println!("Initial state:");
        // print_trie_state(&trie);

        // 测试删除
        let doc = docs_map.get(&1).unwrap(); // apple
        trie.lock().unwrap().delete(doc);
        // 打印删除后的状态
        // println!("After deletion:");
        // print_trie_state(&trie);

        let result = trie.lock().unwrap().search("apple", 5, None);
        assert_eq!(result.len(), 0);

        let result = trie.lock().unwrap().search("app", 5, None);
        assert_eq!(result, vec![5, 4, 2, 3]);

        let result = trie.lock().unwrap().search("l", 5, None);
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_timestamp_ids() {
        let trie = Trie::new();
        let (_, docs_vals) = get_test_docs();
        trie.insert_list(&docs_vals);
        assert_eq!(trie.timestamp_ids.lock().unwrap().len(), 10);
    }

    #[test]
    fn test_timestamp_ids_lt() {
        let trie = Arc::new(Mutex::new(Trie::new()));
        let (docs_map, docs_vals) = get_test_docs();
        trie.lock().unwrap().insert_list(&docs_vals);

        let mut result: Vec<i32> = trie.lock().unwrap().td_lt_ids(6).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![1, 2, 3]);

        // 删除 apple
        let doc = docs_map.get(&1).unwrap();
        trie.lock().unwrap().delete(doc);

        let mut result: Vec<i32> = trie.lock().unwrap().td_lt_ids(6).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_timestamp_ids_gt() {
        let trie = Arc::new(Mutex::new(Trie::new()));
        let (docs_map, docs_vals) = get_test_docs();
        trie.lock().unwrap().insert_list(&docs_vals);

        let mut result: Vec<i32> = trie.lock().unwrap().td_gt_ids(9).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![8, 9, 10]);

        // 删除 8:  应用
        let doc = docs_map.get(&8).unwrap();
        trie.lock().unwrap().delete(doc);

        let mut result: Vec<i32> = trie.lock().unwrap().td_gt_ids(10).into_iter().collect();
        result.sort();
        assert_eq!(result, vec![9, 10]);
    }

    #[test]
    fn test_case_insensitive_search() {
        let trie = Trie::new();
        let (_, docs_vals) = get_test_docs();
        trie.insert_list(&docs_vals);

        // 测试大写搜索
        let results = trie.search("APPLE", 5, None);
        assert_eq!(results, vec![1]);

        // 测试混合大小写搜索
        let results = trie.search("ApPlIcAtIoN", 5, None);
        assert_eq!(results, vec![2]);

        // 测试前缀匹配，大写
        let results = trie.search("APP", 5, None);
        assert_eq!(results, vec![5, 4, 2, 3, 1]);

        // 测试中文不受影响
        let results = trie.search("苹果公司", 5, None);
        assert_eq!(results, vec![7]);

        // 测试带有 doc_type 的大小写不敏感搜索
        let results = trie.search("APPly", 5, Some(vec![2]));
        assert_eq!(results, vec![3]);
    }

    #[test]
    fn test_case_insensitive_delete() {
        let trie = Arc::new(Mutex::new(Trie::new()));
        let (docs_map, docs_vals) = get_test_docs();
        trie.lock().unwrap().insert_list(&docs_vals);

        // 测试删除（使用大写）
        let mut upper_case_doc = docs_map.get(&1).unwrap().clone(); // apple
        upper_case_doc.content = upper_case_doc.content.to_uppercase();
        trie.lock().unwrap().delete(&upper_case_doc);

        let result = trie.lock().unwrap().search("apple", 5, None);
        assert_eq!(result.len(), 0);

        let result = trie.lock().unwrap().search("APP", 5, None);
        assert_eq!(result, vec![5, 4, 2, 3]);
    }

    #[test]
    fn test_search_type_list() {
        let trie = Trie::new();
        let (_, docs_vals) = get_test_docs();
        trie.insert_list(&docs_vals);

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
        let trie = Trie::new();
        let (_, docs_vals) = get_bad_case();
        trie.insert_list(&docs_vals);

        let result = trie.search("i", 4, None);
        assert_eq!(result, vec![3, 2, 1])
    }
}
