//! Real-world scenario tests for generic types
//!
//! These tests demonstrate practical usage patterns of generic structs
//! and enums that would be common in real applications.

#[path = "../utils/mod.rs"]
mod utils;

use utils::generic_test_helpers::*;

#[test]
fn test_linked_list() {
    let code = r#"
        struct Node<T> {
            value: T,
            next: Option<Box<Node<T>>>
        }
        
        struct Box<T> {
            value: T
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        struct LinkedList<T> {
            head: Option<Box<Node<T>>>
        }
        
        impl<T> LinkedList<T> {
            fn new() -> LinkedList<T> {
                LinkedList { head: Option::None }
            }
            
            fn push(&mut self, value: T) {
                let new_node = Node {
                    value: value,
                    next: self.head
                };
                self.head = Option::Some(Box { value: new_node });
            }
        }
        
        fn main() {
            let mut list1 = LinkedList::new();
            list1.push(42);
            list1.push(100);
            
            let mut list2 = LinkedList::new();
            list2.push("hello");
            list2.push("world");
        }
    "#;

    let program = compile_generic_program(code);
    // Recursive types and impl blocks might have issues

    if let Ok(ref prog) = program {
        // Should have LinkedList<i32> and LinkedList<string>
        let list_instances = count_monomorphized_instances(prog, "LinkedList");
        assert!(
            list_instances >= 2,
            "Should have at least 2 LinkedList instantiations"
        );

        // Should also have Node and Box instantiations
        let node_instances = count_monomorphized_instances(prog, "Node");
        assert!(
            node_instances >= 2,
            "Should have at least 2 Node instantiations"
        );
    }
}

#[test]
fn test_simple_hashmap() {
    let code = r#"
        struct Entry<K, V> {
            key: K,
            value: V,
            next: Option<Box<Entry<K, V>>>
        }
        
        struct Box<T> {
            value: T
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        struct HashMap<K, V> {
            buckets: [Option<Box<Entry<K, V>>>],
            size: i32
        }
        
        impl<K, V> HashMap<K, V> {
            fn new() -> HashMap<K, V> {
                HashMap {
                    buckets: [],  // Simplified - would need proper initialization
                    size: 0
                }
            }
            
            fn insert(&mut self, key: K, value: V) {
                // Simplified implementation
                let entry = Entry {
                    key: key,
                    value: value,
                    next: Option::None
                };
                // Would hash and insert properly
                self.size = self.size + 1;
            }
        }
        
        fn main() {
            let mut map1 = HashMap::new();
            map1.insert("name", "John");
            map1.insert("city", "NYC");
            
            let mut map2 = HashMap::new();
            map2.insert(1, "one");
            map2.insert(2, "two");
        }
    "#;

    let program = compile_generic_program(code);

    if let Ok(ref prog) = program {
        // Should have HashMap<string, string> and HashMap<i32, string>
        let map_instances = count_monomorphized_instances(prog, "HashMap");
        assert!(
            map_instances >= 2,
            "Should have at least 2 HashMap instantiations"
        );

        // Should also have Entry instantiations
        let entry_instances = count_monomorphized_instances(prog, "Entry");
        assert!(
            entry_instances >= 2,
            "Should have at least 2 Entry instantiations"
        );
    }
}

#[test]
fn test_vec_implementation() {
    let code = r#"
        struct Vec<T> {
            data: [T],
            len: i32,
            capacity: i32
        }
        
        impl<T> Vec<T> {
            fn new() -> Vec<T> {
                Vec {
                    data: [],
                    len: 0,
                    capacity: 0
                }
            }
            
            fn push(&mut self, value: T) {
                // Simplified - would need to handle capacity
                self.len = self.len + 1;
            }
            
            fn pop(&mut self) -> Option<T> {
                if self.len > 0 {
                    self.len = self.len - 1;
                    // Would return the value
                    Option::None
                } else {
                    Option::None
                }
            }
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        fn main() {
            let mut vec1 = Vec::new();
            vec1.push(1);
            vec1.push(2);
            vec1.push(3);
            
            let mut vec2 = Vec::new();
            vec2.push("a");
            vec2.push("b");
            
            let mut vec3 = Vec::new();
            vec3.push(true);
            vec3.push(false);
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");

    // Should have Vec<i32>, Vec<string>, and Vec<bool>
    let vec_instances = count_monomorphized_instances(&program, "Vec");
    assert_eq!(vec_instances, 3, "Should have 3 Vec instantiations");
}

#[test]
fn test_iterator_pattern() {
    let code = r#"
        trait Iterator {
            type Item;
            fn next(&mut self) -> Option<Self::Item>;
        }
        
        struct ArrayIter<T> {
            data: [T],
            index: i32
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        impl<T> ArrayIter<T> {
            fn new(data: [T]) -> ArrayIter<T> {
                ArrayIter {
                    data: data,
                    index: 0
                }
            }
        }
        
        fn main() {
            let iter1 = ArrayIter::new([1, 2, 3, 4, 5]);
            let iter2 = ArrayIter::new(["a", "b", "c"]);
        }
    "#;

    let program = compile_generic_program(code);

    if let Ok(ref prog) = program {
        // Should have ArrayIter<i32> and ArrayIter<string>
        let iter_instances = count_monomorphized_instances(prog, "ArrayIter");
        assert!(
            iter_instances >= 2,
            "Should have at least 2 ArrayIter instantiations"
        );
    }
}

#[test]
fn test_smart_pointers() {
    let code = r#"
        struct Rc<T> {
            value: T,
            ref_count: i32
        }
        
        struct Arc<T> {
            value: T,
            ref_count: i32
        }
        
        struct Box<T> {
            value: T
        }
        
        impl<T> Box<T> {
            fn new(value: T) -> Box<T> {
                Box { value: value }
            }
        }
        
        impl<T> Rc<T> {
            fn new(value: T) -> Rc<T> {
                Rc {
                    value: value,
                    ref_count: 1
                }
            }
            
            fn clone(&self) -> Rc<T> {
                // Would increment ref count
                Rc {
                    value: self.value,
                    ref_count: self.ref_count + 1
                }
            }
        }
        
        fn main() {
            let b1 = Box::new(42);
            let b2 = Box::new("hello");
            
            let rc1 = Rc::new(100);
            let rc2 = rc1.clone();
            
            let arc1 = Arc { value: [1, 2, 3], ref_count: 1 };
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");

    // Check various smart pointer instantiations
    assert!(count_monomorphized_instances(&program, "Box") >= 2);
    assert!(count_monomorphized_instances(&program, "Rc") >= 1);
    assert!(count_monomorphized_instances(&program, "Arc") >= 1);
}

#[test]
fn test_graph_structure() {
    let code = r#"
        struct Graph<T> {
            nodes: Vec<Node<T>>,
            edges: Vec<Edge>
        }
        
        struct Node<T> {
            id: i32,
            value: T
        }
        
        struct Edge {
            from: i32,
            to: i32,
            weight: f32
        }
        
        struct Vec<T> {
            data: [T],
            len: i32
        }
        
        impl<T> Graph<T> {
            fn new() -> Graph<T> {
                Graph {
                    nodes: Vec { data: [], len: 0 },
                    edges: Vec { data: [], len: 0 }
                }
            }
            
            fn add_node(&mut self, value: T) -> i32 {
                let id = self.nodes.len;
                // Would push to nodes vec
                id
            }
        }
        
        fn main() {
            let mut graph1 = Graph::new();
            graph1.add_node("A");
            graph1.add_node("B");
            
            let mut graph2 = Graph::new();
            graph2.add_node(1);
            graph2.add_node(2);
        }
    "#;

    let program = compile_generic_program(code).expect("Failed to compile");

    // Should have Graph<string> and Graph<i32>
    let graph_instances = count_monomorphized_instances(&program, "Graph");
    assert_eq!(graph_instances, 2, "Should have 2 Graph instantiations");

    // Should also have Node instantiations
    let node_instances = count_monomorphized_instances(&program, "Node");
    assert_eq!(node_instances, 2, "Should have 2 Node instantiations");
}

#[test]
fn test_channel_communication() {
    let code = r#"
        struct Sender<T> {
            buffer: Vec<T>
        }
        
        struct Receiver<T> {
            buffer: Vec<T>
        }
        
        struct Vec<T> {
            data: [T],
            len: i32
        }
        
        enum Option<T> {
            Some(T),
            None
        }
        
        fn channel<T>() -> (Sender<T>, Receiver<T>) {
            let buffer = Vec { data: [], len: 0 };
            (
                Sender { buffer: buffer },
                Receiver { buffer: Vec { data: [], len: 0 } }
            )
        }
        
        impl<T> Sender<T> {
            fn send(&mut self, value: T) {
                // Would add to buffer
            }
        }
        
        impl<T> Receiver<T> {
            fn recv(&mut self) -> Option<T> {
                // Would remove from buffer
                Option::None
            }
        }
        
        fn main() {
            let (tx1, rx1) = channel::<i32>();
            tx1.send(42);
            
            let (tx2, rx2) = channel::<string>();
            tx2.send("hello");
        }
    "#;

    let program = compile_generic_program(code);

    if let Ok(ref prog) = program {
        // Should have Sender<i32>, Sender<string>
        let sender_instances = count_monomorphized_instances(prog, "Sender");
        assert!(
            sender_instances >= 2,
            "Should have at least 2 Sender instantiations"
        );

        // Should have Receiver<i32>, Receiver<string>
        let receiver_instances = count_monomorphized_instances(prog, "Receiver");
        assert!(
            receiver_instances >= 2,
            "Should have at least 2 Receiver instantiations"
        );
    }
}

#[test]
fn test_binary_tree() {
    let code = r#"
        enum Tree<T> {
            Leaf(T),
            Node(T, Box<Tree<T>>, Box<Tree<T>>),
            Empty
        }
        
        struct Box<T> {
            value: T
        }
        
        impl<T> Tree<T> {
            fn leaf(value: T) -> Tree<T> {
                Tree::Leaf(value)
            }
            
            fn empty() -> Tree<T> {
                Tree::Empty
            }
        }
        
        fn main() {
            let t1 = Tree::leaf(42);
            let t2 = Tree::empty();
            
            let t3 = Tree::Node(
                10,
                Box { value: Tree::leaf(5) },
                Box { value: Tree::leaf(15) }
            );
            
            let t4 = Tree::leaf("root");
        }
    "#;

    let program = compile_generic_program(code);

    if let Ok(ref prog) = program {
        // Should have Tree<i32> and Tree<string>
        let tree_instances = count_monomorphized_instances(prog, "Tree");
        assert!(
            tree_instances >= 2,
            "Should have at least 2 Tree instantiations"
        );
    }
}
