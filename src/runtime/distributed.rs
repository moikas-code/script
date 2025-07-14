use crate::runtime::closure::Closure;
use crate::runtime::value::Value;
use crate::runtime::{RuntimeError, ScriptRc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// Distributed closure execution protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributedMessage {
    /// Execute a closure remotely
    ExecuteClosure {
        closure_id: String,
        serialized_closure: Vec<u8>,
        args: Vec<SerializedValue>,
    },
    /// Result of remote execution
    ExecutionResult {
        closure_id: String,
        result: Result<SerializedValue, String>,
    },
    /// Heartbeat to check connection
    Heartbeat,
    /// Acknowledgment
    Ack,
}

/// Serializable representation of Script values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedValue {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Array(Vec<SerializedValue>),
    Object(HashMap<String, SerializedValue>),
}

impl SerializedValue {
    /// Convert from runtime Value
    pub fn from_value(value: &Value) -> Result<Self, RuntimeError> {
        match value {
            Value::Null => Ok(SerializedValue::Null),
            Value::Bool(b) => Ok(SerializedValue::Bool(*b)),
            Value::I32(i) => Ok(SerializedValue::I32(*i)),
            Value::I64(i) => Ok(SerializedValue::I64(*i)),
            Value::F32(f) => Ok(SerializedValue::F32(*f)),
            Value::F64(f) => Ok(SerializedValue::F64(*f)),
            Value::String(s) => Ok(SerializedValue::String(s.clone())),
            Value::Array(arr) => {
                let mut serialized = Vec::new();
                for item in arr.iter() {
                    serialized.push(SerializedValue::from_value(item)?);
                }
                Ok(SerializedValue::Array(serialized))
            }
            Value::Object(obj) => {
                let mut serialized = HashMap::new();
                for (k, v) in obj.iter() {
                    serialized.insert(k.clone(), SerializedValue::from_value(v)?);
                }
                Ok(SerializedValue::Object(serialized))
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Cannot serialize this value type for distributed execution".to_string(),
            )),
        }
    }

    /// Convert to runtime Value
    pub fn to_value(&self) -> Value {
        match self {
            SerializedValue::Null => Value::Null,
            SerializedValue::Bool(b) => Value::Bool(*b),
            SerializedValue::I32(i) => Value::I32(*i),
            SerializedValue::I64(i) => Value::I64(*i),
            SerializedValue::F32(f) => Value::F32(*f),
            SerializedValue::F64(f) => Value::F64(*f),
            SerializedValue::String(s) => Value::String(s.clone()),
            SerializedValue::Array(arr) => {
                let values: Vec<ScriptRc<Value>> =
                    arr.iter().map(|v| ScriptRc::new(v.to_value())).collect();
                Value::Array(values)
            }
            SerializedValue::Object(obj) => {
                let mut values = HashMap::new();
                for (k, v) in obj.iter() {
                    values.insert(k.clone(), ScriptRc::new(v.to_value()));
                }
                Value::Object(values)
            }
        }
    }
}

/// Distributed execution node
pub struct DistributedNode {
    /// Node identifier
    node_id: String,
    /// Address to listen on
    address: String,
    /// Active connections to other nodes
    connections: Arc<Mutex<HashMap<String, TcpStream>>>,
    /// Pending tasks
    pending_tasks: Arc<Mutex<HashMap<String, PendingTask>>>,
}

/// A task pending execution result
struct PendingTask {
    closure_id: String,
    start_time: std::time::Instant,
}

impl DistributedNode {
    /// Create a new distributed node
    pub fn new(node_id: String, address: String) -> Self {
        Self {
            node_id,
            address,
            connections: Arc::new(Mutex::new(HashMap::new())),
            pending_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start listening for incoming connections
    pub fn start_listening(&self) -> Result<(), RuntimeError> {
        let listener = TcpListener::bind(&self.address)
            .map_err(|e| RuntimeError::InvalidOperation(format!("Failed to bind: {e}")))?;

        let connections = Arc::clone(&self.connections);
        let pending_tasks = Arc::clone(&self.pending_tasks);

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let connections = Arc::clone(&connections);
                        let pending_tasks = Arc::clone(&pending_tasks);

                        thread::spawn(move || {
                            handle_connection(&mut stream, connections, pending_tasks);
                        });
                    }
                    Err(e) => eprintln!("Connection failed: {e}"),
                }
            }
        });

        Ok(())
    }

    /// Connect to another node
    pub fn connect_to(&self, node_address: &str) -> Result<(), RuntimeError> {
        let stream = TcpStream::connect(node_address)
            .map_err(|e| RuntimeError::InvalidOperation(format!("Failed to connect: {e}")))?;

        let mut connections = self.connections.lock().unwrap();
        connections.insert(node_address.to_string(), stream);

        Ok(())
    }

    /// Execute a closure on a remote node
    pub fn remote_execute(
        &self,
        node_address: &str,
        _closure: &Closure,
        args: &[Value],
    ) -> Result<String, RuntimeError> {
        let closure_id = format!("closure_{uuid::Uuid::new_v4(}"));

        // Serialize closure (simplified - in reality would need proper serialization)
        let serialized_closure = vec![]; // TODO: Implement closure serialization

        // Serialize arguments
        let mut serialized_args = Vec::new();
        for arg in args {
            serialized_args.push(SerializedValue::from_value(arg)?);
        }

        let message = DistributedMessage::ExecuteClosure {
            closure_id: closure_id.clone(),
            serialized_closure,
            args: serialized_args,
        };

        // Send message to remote node
        let mut connections = self.connections.lock().unwrap();
        if let Some(stream) = connections.get_mut(node_address) {
            let serialized = serde_json::to_vec(&message).map_err(|e| {
                RuntimeError::InvalidOperation(format!("Serialization error: {e}"))
            })?;

            stream
                .write_all(&serialized)
                .map_err(|e| RuntimeError::InvalidOperation(format!("Write error: {e}")))?;

            // Track pending task
            let mut pending = self.pending_tasks.lock().unwrap();
            pending.insert(
                closure_id.clone(),
                PendingTask {
                    closure_id: closure_id.clone(),
                    start_time: std::time::Instant::now(),
                },
            );

            Ok(closure_id)
        } else {
            Err(RuntimeError::InvalidOperation(format!(
                "Not connected to node: {}",
                node_address
            )))
        }
    }
}

/// Handle incoming connections
fn handle_connection(
    stream: &mut TcpStream,
    _connections: Arc<Mutex<HashMap<String, TcpStream>>>,
    _pending_tasks: Arc<Mutex<HashMap<String, PendingTask>>>,
) {
    let mut buffer = Vec::new();
    match stream.read_to_end(&mut buffer) {
        Ok(_) => {
            match serde_json::from_slice::<DistributedMessage>(&buffer) {
                Ok(message) => {
                    match message {
                        DistributedMessage::ExecuteClosure { closure_id, .. } => {
                            // Execute closure and send result back
                            let result = SerializedValue::String("Result".to_string());
                            let response = DistributedMessage::ExecutionResult {
                                closure_id,
                                result: Ok(result),
                            };

                            if let Ok(serialized) = serde_json::to_vec(&response) {
                                let _ = stream.write_all(&serialized);
                            }
                        }
                        DistributedMessage::Heartbeat => {
                            let response = DistributedMessage::Ack;
                            if let Ok(serialized) = serde_json::to_vec(&response) {
                                let _ = stream.write_all(&serialized);
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => eprintln!("Failed to deserialize message: {e}"),
            }
        }
        Err(e) => eprintln!("Failed to read from stream: {e}"),
    }
}

/// Distributed task scheduler
pub struct DistributedScheduler {
    /// Nodes in the cluster
    nodes: Vec<DistributedNode>,
    /// Load balancing strategy
    strategy: LoadBalancingStrategy,
}

#[derive(Debug, Clone)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Least loaded node
    LeastLoaded,
    /// Random distribution
    Random,
}

impl DistributedScheduler {
    /// Create a new distributed scheduler
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            nodes: Vec::new(),
            strategy,
        }
    }

    /// Add a node to the cluster
    pub fn add_node(&mut self, node: DistributedNode) {
        self.nodes.push(node);
    }

    /// Schedule a closure for execution
    pub fn schedule(&self, closure: &Closure, args: &[Value]) -> Result<String, RuntimeError> {
        if self.nodes.is_empty() {
            return Err(RuntimeError::InvalidOperation(
                "No nodes available".to_string(),
            ));
        }

        // Select node based on strategy
        let node_index = match self.strategy {
            LoadBalancingStrategy::RoundRobin => 0, // TODO: Implement proper round-robin
            LoadBalancingStrategy::LeastLoaded => 0, // TODO: Implement load tracking
            LoadBalancingStrategy::Random => {
                use rand::Rng;
                rand::thread_rng().gen_range(0..self.nodes.len())
            }
        };

        let node = &self.nodes[node_index];
        node.remote_execute(&node.address, closure, args)
    }
}

/// Standard library function implementations for distributed computing

/// Implementation of remote_execute for stdlib registry
pub(crate) fn remote_execute_impl(
    args: &[crate::stdlib::ScriptValue],
) -> Result<crate::stdlib::ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "remote_execute expects 3 arguments, got {}",
            args.len()
        )));
    }

    // For now, return a placeholder future
    Ok(crate::stdlib::ScriptValue::String(
        crate::runtime::ScriptRc::new(crate::stdlib::string::ScriptString::from("RemoteFuture")),
    ))
}

/// Implementation of distribute_map for stdlib registry
pub(crate) fn distribute_map_impl(
    args: &[crate::stdlib::ScriptValue],
) -> Result<crate::stdlib::ScriptValue, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "distribute_map expects 2 arguments, got {}",
            args.len()
        )));
    }

    // For now, return the original array
    Ok(args[0].clone())
}

/// Implementation of cluster_reduce for stdlib registry
pub(crate) fn cluster_reduce_impl(
    args: &[crate::stdlib::ScriptValue],
) -> Result<crate::stdlib::ScriptValue, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "cluster_reduce expects 3 arguments, got {}",
            args.len()
        )));
    }

    // For now, return the initial value
    Ok(args[2].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialized_value_conversion() {
        let value = Value::I32(42);
        let serialized = SerializedValue::from_value(&value).unwrap();
        assert!(matches!(serialized, SerializedValue::I32(42)));

        let deserialized = serialized.to_value();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn test_distributed_node_creation() {
        let node = DistributedNode::new("node1".to_string(), "127.0.0.1:8080".to_string());
        assert_eq!(node.node_id, "node1");
        assert_eq!(node.address, "127.0.0.1:8080");
    }

    #[test]
    fn test_distributed_scheduler() {
        let scheduler = DistributedScheduler::new(LoadBalancingStrategy::RoundRobin);
        assert_eq!(scheduler.nodes.len(), 0);
    }
}
