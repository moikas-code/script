# Audit Logging Specification

**Version**: 1.0  
**Status**: Design Phase  
**Compliance**: SOC2, ISO 27001, GDPR

## Overview

This specification defines the audit logging requirements for Script to achieve SOC2 compliance and support security monitoring, incident response, and compliance reporting.

## Core Requirements

### What Must Be Logged

#### Authentication Events
```json
{
  "event_type": "auth",
  "timestamp": "2025-01-09T10:15:30.123Z",
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "user123",
  "ip_address": "192.168.1.100",
  "user_agent": "Script-CLI/0.5.0",
  "action": "login",
  "result": "success",
  "mfa_used": true,
  "session_id": "sess_123abc"
}
```

#### Authorization Events
```json
{
  "event_type": "authz",
  "timestamp": "2025-01-09T10:16:45.789Z",
  "event_id": "660e8400-e29b-41d4-a716-446655440001",
  "user_id": "user123",
  "session_id": "sess_123abc",
  "resource": "module:stdlib/crypto",
  "action": "execute",
  "permission": "script.module.execute",
  "result": "denied",
  "reason": "insufficient_privileges"
}
```

#### Data Access Events
```json
{
  "event_type": "data_access",
  "timestamp": "2025-01-09T10:17:00.000Z",
  "event_id": "770e8400-e29b-41d4-a716-446655440002",
  "user_id": "user123",
  "session_id": "sess_123abc",
  "resource_type": "file",
  "resource_id": "/home/user/data.script",
  "action": "read",
  "bytes_accessed": 1024,
  "result": "success"
}
```

#### System Events
```json
{
  "event_type": "system",
  "timestamp": "2025-01-09T10:18:00.000Z",
  "event_id": "880e8400-e29b-41d4-a716-446655440003",
  "action": "config_change",
  "component": "runtime",
  "setting": "max_memory_limit",
  "old_value": "1GB",
  "new_value": "2GB",
  "changed_by": "admin",
  "approval_id": "CHG-2025-001"
}
```

#### Security Events
```json
{
  "event_type": "security",
  "timestamp": "2025-01-09T10:19:00.000Z",
  "event_id": "990e8400-e29b-41d4-a716-446655440004",
  "severity": "high",
  "action": "intrusion_attempt",
  "source_ip": "10.0.0.100",
  "target": "compiler",
  "attack_type": "buffer_overflow",
  "blocked": true,
  "rule_id": "SEC-001"
}
```

#### Error Events
```json
{
  "event_type": "error",
  "timestamp": "2025-01-09T10:20:00.000Z",
  "event_id": "aa0e8400-e29b-41d4-a716-446655440005",
  "severity": "error",
  "component": "parser",
  "error_code": "PARSE_001",
  "message": "Unexpected token",
  "file": "main.script",
  "line": 42,
  "column": 15,
  "stack_trace": "..."
}
```

## Log Format Standards

### Required Fields (All Events)
- `event_type`: Category of event
- `timestamp`: ISO 8601 UTC timestamp with milliseconds
- `event_id`: UUID v4 for correlation
- `version`: Log format version

### Contextual Fields
- `user_id`: User identifier (if applicable)
- `session_id`: Session identifier
- `request_id`: Request correlation ID
- `trace_id`: Distributed trace ID

### Security Fields
- `ip_address`: Source IP (anonymized for GDPR)
- `user_agent`: Client identifier
- `severity`: critical|high|medium|low|info
- `result`: success|failure|error

## Storage Requirements

### Retention Policies
| Event Type | Retention Period | Storage Type |
|------------|-----------------|--------------|
| Authentication | 18 months | Encrypted |
| Authorization | 12 months | Encrypted |
| Data Access | 12 months | Encrypted |
| System Changes | 7 years | Encrypted + Archive |
| Security Events | 24 months | Encrypted + WORM |
| Errors | 6 months | Compressed |

### Storage Architecture
```
┌─────────────────┐
│   Application   │
└────────┬────────┘
         │
┌────────▼────────┐
│   Log Buffer    │ ← In-memory ring buffer
└────────┬────────┘
         │
┌────────▼────────┐
│  Log Processor  │ ← Filtering, anonymization
└────────┬────────┘
         │
    ┌────┴────┬─────────┬──────────┐
    │         │         │          │
┌───▼──┐ ┌───▼──┐ ┌────▼───┐ ┌───▼───┐
│ File │ │ SIEM │ │ S3/GCS │ │ Splunk│
└──────┘ └──────┘ └────────┘ └───────┘
```

## Privacy & Compliance

### GDPR Compliance
- PII fields must be marked: `"pii": true`
- Support right to erasure
- Anonymization after retention period
- Consent tracking for data processing

### Data Anonymization
```rust
fn anonymize_ip(ip: &str) -> String {
    // IPv4: 192.168.1.100 -> 192.168.1.0
    // IPv6: 2001:db8::1 -> 2001:db8::
}

fn hash_user_id(id: &str, salt: &[u8]) -> String {
    // One-way hash with daily rotating salt
}
```

### Field Encryption
Sensitive fields must be encrypted:
- User identifiers (after anonymization period)
- IP addresses (after 24 hours)
- File paths containing user data
- Any field marked as sensitive

## Implementation Guidelines

### Performance Requirements
- Max 5% overhead on operations
- Async logging to prevent blocking
- Batch writes every 100ms or 1000 events
- Compression for storage efficiency

### Reliability
- At-least-once delivery guarantee
- Local buffer for network failures
- Graceful degradation under load
- No lost events during shutdown

### Integration Points
```rust
// Core trait for audit logging
pub trait Auditable {
    fn to_audit_event(&self) -> AuditEvent;
}

// Automatic instrumentation
#[audit_log]
fn sensitive_operation() -> Result<()> {
    // Automatically logged
}

// Manual logging
audit_logger.log(AuditEvent::custom()
    .event_type("custom_event")
    .add_field("key", "value")
    .build());
```

## Monitoring & Alerting

### Real-time Alerts
- Failed authentication attempts > 5 in 5 minutes
- Privilege escalation attempts
- Access to sensitive resources
- Configuration changes
- System errors > threshold

### Compliance Reports
- Daily summary of access patterns
- Weekly security event report
- Monthly compliance dashboard
- Quarterly audit report

## Testing Requirements

### Unit Tests
- Log format validation
- Field encryption/decryption
- Anonymization functions
- Buffer overflow handling

### Integration Tests
- End-to-end event flow
- Storage backend failover
- Performance benchmarks
- Compliance validation

### Audit Simulation
- Generate 6 months of logs
- Verify retention policies
- Test data export capabilities
- Validate report generation

## Rollout Plan

### Phase 1: Core Infrastructure
- Implement logging framework
- Add buffer and processor
- Basic file storage

### Phase 2: Event Types
- Authentication events
- Authorization events
- Error events

### Phase 3: Integration
- SIEM integration
- Cloud storage
- Monitoring dashboards

### Phase 4: Compliance
- Encryption implementation
- Anonymization rules
- Retention automation
- Audit reports

---

**Note**: This specification must be reviewed by legal and compliance teams before implementation.