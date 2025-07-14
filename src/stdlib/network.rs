//! Network I/O operations for the Script programming language
//!
//! This module provides basic TCP and UDP networking functionality
//! including client connections, server listeners, and data transfer.
//!
//! All functions are designed to be called from Script code and handle
//! errors using the Script Result type.

use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::error::{io_error_from_std, IoError, IoErrorKind, ScriptError};
use crate::stdlib::{ScriptResult, ScriptString, ScriptValue};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

/// TCP connection handle for Script
pub struct ScriptTcpStream {
    stream: TcpStream,
}

impl ScriptTcpStream {
    /// Create a new TCP stream from a standard TcpStream
    pub fn new(stream: TcpStream) -> Self {
        ScriptTcpStream { stream }
    }

    /// Connect to a TCP server
    pub fn connect(addr: &str) -> Result<Self, IoError> {
        match TcpStream::connect(addr) {
            Ok(stream) => Ok(ScriptTcpStream::new(stream)),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to connect to '{}': {addr, io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Connect with a timeout
    pub fn connect_timeout(addr: &str, timeout_ms: u64) -> Result<Self, IoError> {
        let timeout = Duration::from_millis(timeout_ms);

        // Parse socket address
        let socket_addr = match addr.parse::<std::net::SocketAddr>() {
            Ok(addr) => addr,
            Err(_) => {
                // Try to resolve the address
                match addr.to_socket_addrs() {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            addr
                        } else {
                            return Err(IoError {
                                message: format!("Failed to resolve address: {addr}"),
                                kind: IoErrorKind::InvalidInput,
                                code: None,
                            });
                        }
                    }
                    Err(e) => {
                        let mut io_err = io_error_from_std(e);
                        io_err.message =
                            format!("Failed to resolve address '{}': {addr, io_err.message}");
                        return Err(io_err);
                    }
                }
            }
        };

        match TcpStream::connect_timeout(&socket_addr, timeout) {
            Ok(stream) => Ok(ScriptTcpStream::new(stream)),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!(
                    "Failed to connect to '{}' with timeout: {}",
                    addr, io_err.message
                );
                Err(io_err)
            }
        }
    }

    /// Read data from the TCP stream
    pub fn read(&mut self, max_bytes: usize) -> Result<Vec<u8>, IoError> {
        let mut buffer = vec![0u8; max_bytes];
        match self.stream.read(&mut buffer) {
            Ok(n) => {
                buffer.truncate(n);
                Ok(buffer)
            }
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to read from TCP stream: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Read a line from the TCP stream
    pub fn read_line(&mut self) -> Result<String, IoError> {
        use std::io::BufRead;
        use std::io::BufReader;

        let mut reader = BufReader::new(&mut self.stream);
        let mut line = String::new();

        match reader.read_line(&mut line) {
            Ok(_) => {
                // Remove trailing newline
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Ok(line)
            }
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to read line from TCP stream: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Write data to the TCP stream
    pub fn write(&mut self, data: &[u8]) -> Result<usize, IoError> {
        match self.stream.write(data) {
            Ok(n) => {
                if let Err(e) = self.stream.flush() {
                    let mut io_err = io_error_from_std(e);
                    io_err.message = format!("Failed to flush TCP stream: {io_err.message}");
                    return Err(io_err);
                }
                Ok(n)
            }
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to write to TCP stream: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Write a string to the TCP stream
    pub fn write_string(&mut self, data: &str) -> Result<usize, IoError> {
        self.write(data.as_bytes())
    }

    /// Set read timeout
    pub fn set_read_timeout(&mut self, timeout_ms: Option<u64>) -> Result<(), IoError> {
        let timeout = timeout_ms.map(Duration::from_millis);
        match self.stream.set_read_timeout(timeout) {
            Ok(()) => Ok(()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to set read timeout: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Set write timeout
    pub fn set_write_timeout(&mut self, timeout_ms: Option<u64>) -> Result<(), IoError> {
        let timeout = timeout_ms.map(Duration::from_millis);
        match self.stream.set_write_timeout(timeout) {
            Ok(()) => Ok(()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to set write timeout: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<String, IoError> {
        match self.stream.local_addr() {
            Ok(addr) => Ok(addr.to_string()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to get local address: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Get the peer address
    pub fn peer_addr(&self) -> Result<String, IoError> {
        match self.stream.peer_addr() {
            Ok(addr) => Ok(addr.to_string()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to get peer address: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Shutdown the connection
    pub fn shutdown(&mut self, how: &str) -> Result<(), IoError> {
        use std::net::Shutdown;

        let shutdown_type = match how {
            "read" => Shutdown::Read,
            "write" => Shutdown::Write,
            "both" => Shutdown::Both,
            _ => {
                return Err(IoError {
                    message: format!(
                        "Invalid shutdown type '{}'. Use 'read', 'write', or 'both'",
                        how
                    ),
                    kind: IoErrorKind::InvalidInput,
                    code: None,
                });
            }
        };

        match self.stream.shutdown(shutdown_type) {
            Ok(()) => Ok(()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to shutdown connection: {io_err.message}");
                Err(io_err)
            }
        }
    }
}

/// TCP listener handle for Script
pub struct ScriptTcpListener {
    listener: TcpListener,
}

impl ScriptTcpListener {
    /// Create a new TCP listener
    pub fn new(listener: TcpListener) -> Self {
        ScriptTcpListener { listener }
    }

    /// Bind to an address and start listening
    pub fn bind(addr: &str) -> Result<Self, IoError> {
        match TcpListener::bind(addr) {
            Ok(listener) => Ok(ScriptTcpListener::new(listener)),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to bind to '{}': {addr, io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Accept a new connection
    pub fn accept(&self) -> Result<(ScriptTcpStream, String), IoError> {
        match self.listener.accept() {
            Ok((stream, addr)) => Ok((ScriptTcpStream::new(stream), addr.to_string())),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to accept connection: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<String, IoError> {
        match self.listener.local_addr() {
            Ok(addr) => Ok(addr.to_string()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to get local address: {io_err.message}");
                Err(io_err)
            }
        }
    }
}

/// UDP socket handle for Script
pub struct ScriptUdpSocket {
    socket: UdpSocket,
}

impl ScriptUdpSocket {
    /// Create a new UDP socket
    pub fn new(socket: UdpSocket) -> Self {
        ScriptUdpSocket { socket }
    }

    /// Bind to an address
    pub fn bind(addr: &str) -> Result<Self, IoError> {
        match UdpSocket::bind(addr) {
            Ok(socket) => Ok(ScriptUdpSocket::new(socket)),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!(
                    "Failed to bind UDP socket to '{}': {}",
                    addr, io_err.message
                );
                Err(io_err)
            }
        }
    }

    /// Connect to a remote address (for send/recv)
    pub fn connect(&mut self, addr: &str) -> Result<(), IoError> {
        match self.socket.connect(addr) {
            Ok(()) => Ok(()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!(
                    "Failed to connect UDP socket to '{}': {}",
                    addr, io_err.message
                );
                Err(io_err)
            }
        }
    }

    /// Send data to the connected address
    pub fn send(&mut self, data: &[u8]) -> Result<usize, IoError> {
        match self.socket.send(data) {
            Ok(n) => Ok(n),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to send UDP data: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Receive data from the connected address
    pub fn recv(&mut self, max_bytes: usize) -> Result<Vec<u8>, IoError> {
        let mut buffer = vec![0u8; max_bytes];
        match self.socket.recv(&mut buffer) {
            Ok(n) => {
                buffer.truncate(n);
                Ok(buffer)
            }
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to receive UDP data: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Send data to a specific address
    pub fn send_to(&mut self, data: &[u8], addr: &str) -> Result<usize, IoError> {
        match self.socket.send_to(data, addr) {
            Ok(n) => Ok(n),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message =
                    format!("Failed to send UDP data to '{}': {addr, io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Receive data and the sender's address
    pub fn recv_from(&mut self, max_bytes: usize) -> Result<(Vec<u8>, String), IoError> {
        let mut buffer = vec![0u8; max_bytes];
        match self.socket.recv_from(&mut buffer) {
            Ok((n, addr)) => {
                buffer.truncate(n);
                Ok((buffer, addr.to_string()))
            }
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to receive UDP data: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<String, IoError> {
        match self.socket.local_addr() {
            Ok(addr) => Ok(addr.to_string()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to get local address: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Set read timeout
    pub fn set_read_timeout(&mut self, timeout_ms: Option<u64>) -> Result<(), IoError> {
        let timeout = timeout_ms.map(Duration::from_millis);
        match self.socket.set_read_timeout(timeout) {
            Ok(()) => Ok(()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to set read timeout: {io_err.message}");
                Err(io_err)
            }
        }
    }

    /// Set write timeout
    pub fn set_write_timeout(&mut self, timeout_ms: Option<u64>) -> Result<(), IoError> {
        let timeout = timeout_ms.map(Duration::from_millis);
        match self.socket.set_write_timeout(timeout) {
            Ok(()) => Ok(()),
            Err(e) => {
                let mut io_err = io_error_from_std(e);
                io_err.message = format!("Failed to set write timeout: {io_err.message}");
                Err(io_err)
            }
        }
    }
}

// Implementation functions for stdlib registry

/// TCP connect implementation
pub(crate) fn tcp_connect_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "tcp_connect expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(addr) => {
            match ScriptTcpStream::connect(&addr.as_str()) {
                Ok(stream) => {
                    // We need to wrap the TCP stream in a ScriptValue
                    // For now, we'll store it as an Object with metadata
                    let mut stream_obj = std::collections::HashMap::new();
                    stream_obj.insert(
                        "_type".to_string(),
                        ScriptValue::String(ScriptRc::new(ScriptString::from_str("TcpStream"))),
                    );
                    // Store the stream as a boxed pointer (this is a simplified approach)
                    // In a real implementation, we'd need a proper handle system
                    let stream_ptr = Box::into_raw(Box::new(stream)) as usize;
                    stream_obj.insert("_handle".to_string(), ScriptValue::I32(stream_ptr as i32));

                    let result = ScriptResult::ok(ScriptValue::Object(ScriptRc::new(stream_obj)));
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
                Err(io_err) => {
                    let error_val = io_err.to_script_value();
                    let result = ScriptResult::err(error_val);
                    Ok(ScriptValue::Result(ScriptRc::new(result)))
                }
            }
        }
        _ => Err(RuntimeError::InvalidOperation(
            "tcp_connect expects a string address argument".to_string(),
        )),
    }
}

/// TCP bind implementation for server
pub(crate) fn tcp_bind_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "tcp_bind expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(addr) => match ScriptTcpListener::bind(&addr.as_str()) {
            Ok(listener) => {
                let mut listener_obj = std::collections::HashMap::new();
                listener_obj.insert(
                    "_type".to_string(),
                    ScriptValue::String(ScriptRc::new(ScriptString::from_str("TcpListener"))),
                );
                let listener_ptr = Box::into_raw(Box::new(listener)) as usize;
                listener_obj.insert("_handle".to_string(), ScriptValue::I32(listener_ptr as i32));

                let result = ScriptResult::ok(ScriptValue::Object(ScriptRc::new(listener_obj)));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "tcp_bind expects a string address argument".to_string(),
        )),
    }
}

/// UDP bind implementation
pub(crate) fn udp_bind_impl(args: &[ScriptValue]) -> Result<ScriptValue, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(format!(
            "udp_bind expects 1 argument, got {}",
            args.len()
        )));
    }

    match &args[0] {
        ScriptValue::String(addr) => match ScriptUdpSocket::bind(&addr.as_str()) {
            Ok(socket) => {
                let mut socket_obj = std::collections::HashMap::new();
                socket_obj.insert(
                    "_type".to_string(),
                    ScriptValue::String(ScriptRc::new(ScriptString::from_str("UdpSocket"))),
                );
                let socket_ptr = Box::into_raw(Box::new(socket)) as usize;
                socket_obj.insert("_handle".to_string(), ScriptValue::I32(socket_ptr as i32));

                let result = ScriptResult::ok(ScriptValue::Object(ScriptRc::new(socket_obj)));
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
            Err(io_err) => {
                let error_val = io_err.to_script_value();
                let result = ScriptResult::err(error_val);
                Ok(ScriptValue::Result(ScriptRc::new(result)))
            }
        },
        _ => Err(RuntimeError::InvalidOperation(
            "udp_bind expects a string address argument".to_string(),
        )),
    }
}

// Note: In a real implementation, we would need many more implementation functions
// for tcp_read, tcp_write, tcp_accept, udp_send, udp_recv, etc.
// We would also need a proper handle management system to safely store and retrieve
// network objects between Script calls.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_connection_error() {
        // Test connecting to an invalid address
        let result = ScriptTcpStream::connect("invalid_address");
        assert!(result.is_err());
    }

    #[test]
    fn test_udp_socket_bind() {
        // Test binding to a local address
        let result = ScriptUdpSocket::bind("127.0.0.1:0");
        assert!(result.is_ok());

        if let Ok(socket) = result {
            // Check we can get the local address
            let addr_result = socket.local_addr();
            assert!(addr_result.is_ok());
        }
    }

    #[test]
    fn test_tcp_listener_bind() {
        // Test binding to a local address
        let result = ScriptTcpListener::bind("127.0.0.1:0");
        assert!(result.is_ok());

        if let Ok(listener) = result {
            // Check we can get the local address
            let addr_result = listener.local_addr();
            assert!(addr_result.is_ok());
        }
    }
}
