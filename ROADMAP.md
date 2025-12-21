# Loom Project Roadmap

## Overview

High-level timeline and planning for Loom's evolution from basic thread persistence to a high-performance real-time server-client system.

**Vision**: A fast, reliable, feature-rich thread persistence and query system that enables interactive AI workflows.

---

## Phase 1: Foundation (COMPLETE ✅)

**Status**: Completed
**Timeline**: Weeks 1-4

### Goals
- Basic thread persistence to SQLite
- HTTP API for CRUD operations
- Core data models

### Deliverables
- ✅ `ThreadRepository` with CRUD operations
- ✅ SQLite schema and migrations
- ✅ HTTP API handlers
- ✅ Error handling framework
- ✅ Configuration system
- ✅ Health check endpoint

### Dependencies
- None (foundation phase)

### Success Criteria
- ✅ All core operations tested
- ✅ Database integrity validated
- ✅ API contract defined

---

## Phase 2: Query Bridge Integration (CURRENT 🚀)

**Status**: In Progress
**Timeline**: Weeks 5-8

### Goals
- Server-to-client query mechanism via SSE + HTTP
- LLM completion streaming with bi-directional communication
- Integration of ServerQuery into event stream

### Deliverables
- ✅ `ServerQueryManager` for managing pending queries
- ✅ ServerQuery event variant in `LlmStreamEvent`
- ✅ SSE streaming infrastructure
- ✅ HTTP query response endpoint
- ✅ Property-based tests for serialization
- 🚀 LLM proxy integration with query support
- 🚀 Client-facing documentation

### Dependencies
- Phase 1: Basic persistence ✅

### Success Criteria
- [ ] Queries transmitted via SSE during LLM streams
- [ ] Query responses received via HTTP POST
- [ ] No message loss (at-least-once delivery)
- [ ] Latency: <500ms for query response
- [ ] All integration tests passing

### Known Limitations
- Polling-based query delivery (~100-500ms latency)
- Multiple TCP connections (SSE + HTTP)
- No native backpressure mechanism

---

## Phase 3: WebSocket Migration (PLANNED 📋)

**Status**: Planning / Design phase
**Timeline**: Weeks 9-14 (6 weeks)
**Resource**: 1-2 engineers

### Goals
- Replace SSE + HTTP with single WebSocket connection
- Reduce query latency from 100-500ms to <50ms
- Unified message protocol for all communication

### Deliverables
- 📋 WebSocket server implementation (`websocket.rs` skeleton ✅)
- 📋 Phase 3 planning document ✅ (see: [specs/phase3_websocket_planning.md](specs/phase3_websocket_planning.md))
- 📋 Message format specification
- 📋 Protocol definition (ServerQuery, QueryResponse, LlmEvent, Control)
- 📋 WebSocket endpoint: `GET /v1/ws/sessions/{session_id}`
- 📋 Backward compatibility layer (HTTP fallback)
- 📋 Load testing framework (1k-100k concurrent connections)
- 📋 Monitoring / metrics system
- 📋 Gradual rollout strategy

### Implementation Phases

#### 3a: Foundation (Weeks 1-2)
- WebSocket server setup
- Message types and serialization
- Connection lifecycle management
- Unit tests

#### 3b: Core Flow (Weeks 2-3)
- Integration with ServerQueryManager
- LLM event forwarding
- Error recovery
- Integration tests

#### 3c: Hardening (Weeks 3-4)
- Backpressure handling
- Keepalive/heartbeat
- Comprehensive error handling
- Chaos tests

#### 3d: Load Testing (Weeks 4-5)
- Sustained load (1000 conn, 100 qps)
- Burst handling (10k new conn/sec)
- Large message testing (10MB)
- Network instability simulation

#### 3e: Rollout (Weeks 5-6)
- Canary deployment
- Gradual ramp (1% → 100%)
- Client migration
- Real-world monitoring

### Dependencies
- Phase 2: Query bridge ✅
- Phase 2: LLM streaming ✅

### Success Criteria
- [ ] Query latency: p99 <100ms (target <50ms)
- [ ] Error rate: <0.1%
- [ ] Memory per connection: <100KB
- [ ] 100k concurrent connections supported
- [ ] Zero message loss
- [ ] 95%+ client adoption within 1 month
- [ ] Canary deployment: 0 incidents

### Risks
- Network instability handling
- Client compatibility
- Server resource exhaustion
- Data loss on disconnect

### Mitigation
- Comprehensive load testing
- Feature flags for gradual rollout
- Robust error recovery
- Message ACK mechanism

---

## Phase 4: Performance Optimization (FUTURE 🔮)

**Status**: Planned (post-Phase 3)
**Timeline**: Weeks 15-18 (4 weeks)

### Goals
- Further latency reduction (<20ms target)
- Support 100k+ concurrent connections
- Optional binary protocol

### Potential Features
- 🔮 Message compression (permessage-deflate)
- 🔮 Selective message subscriptions
- 🔮 Message batching
- 🔮 Binary protocol option (vs JSON)
- 🔮 Connection pooling
- 🔮 Database query optimization
- 🔮 Caching layer for frequently-accessed data

### Conditional on Phase 3 Results
- If WebSocket achieves targets (<50ms): focus on code cleanup
- If additional optimization needed: implement compression + binary protocol

### Dependencies
- Phase 3: WebSocket ✅ (target)

---

## Phase 5: Enterprise Features (FUTURE 🔮)

**Status**: Planned
**Timeline**: Weeks 19+

### Potential Features
- 🔮 Multi-user collaboration
- 🔮 Thread versioning / branching
- 🔮 Conflict resolution
- 🔮 Audit logging
- 🔮 Access control lists (ACL)
- 🔮 Thread sharing / permissions
- 🔮 Analytics / insights

### Conditional Depending on User Feedback

---

## Cross-Cutting Concerns

### Monitoring & Observability

#### Implemented (Phase 1-2)
- ✅ Structured logging with `tracing`
- ✅ Error categorization

#### Phase 3
- 📋 WebSocket metrics
  - Per-connection: bytes sent/recv, message count, latency
  - Global: total connections, message rate, error rate
- 📋 Performance metrics
  - Query latency (p50, p99, max)
  - Connection establishment time
  - Message delivery success rate
- 📋 Resource metrics
  - Memory per connection
  - CPU usage
  - Thread count
  - Database connections

#### Phase 4+
- 🔮 Distributed tracing
- 🔮 Real-time dashboards
- 🔮 Alerting rules
- 🔮 SLA tracking

### Testing Strategy

#### Unit Tests (All Phases)
- Component behavior
- Error handling
- Edge cases

#### Integration Tests
- Phase 2: Server-client communication
- Phase 3: WebSocket message flow
- Phase 4: Performance benchmarks

#### Load Tests
- Phase 3: Foundation for capacity planning
- Phase 4+: Continuous benchmarking

#### Chaos Tests
- Phase 3: Network instability
- Phase 3: Connection drops
- Phase 4+: Database failures, cascading errors

### Security Considerations

#### Implemented
- ✅ Session authentication
- ✅ Basic error handling (no info leaks)

#### Phase 3
- 📋 WebSocket authentication validation
- 📋 Message validation (no injection)
- 📋 Rate limiting per connection
- 📋 DDoS protection considerations

#### Phase 4+
- 🔮 End-to-end encryption option
- 🔮 Message signing/verification
- 🔮 Audit logging
- 🔮 Access control

### Documentation

#### Current
- ✅ Architecture overview
- ✅ Core systems (streaming, query bridge)
- ✅ API specifications

#### Phase 3
- 📋 WebSocket protocol spec
- 📋 Migration guide for clients
- 📋 Troubleshooting guide

#### Phase 4+
- 🔮 Performance tuning guide
- 🔮 Scaling guide
- 🔮 Deployment best practices

---

## Critical Path & Dependencies

```
Phase 1 (Foundation)
    ↓
Phase 2 (Query Bridge)
    ↓
Phase 3 (WebSocket) ← Most critical for performance
    ↓
Phase 4 (Optimization)
    ↓
Phase 5 (Enterprise)
```

**Critical Path**: Phase 1 → Phase 2 → Phase 3  
**Longest Phase**: Phase 3 (6 weeks including load testing)  
**Resource Bottleneck**: Load testing infrastructure (Phase 3d)

---

## Resource Requirements

### Phase 2 (Current)
- **Team**: 1-2 engineers
- **Infrastructure**: Development machine, CI/CD
- **Testing**: Unit + integration tests
- **Timeline**: 4 weeks

### Phase 3 (WebSocket)
- **Team**: 1-2 engineers
- **Infrastructure**: Load testing environment (100k+ connections)
- **Testing**: Unit + integration + load tests
- **Timeline**: 6 weeks
- **Estimated Cost**: Load testing tools, infrastructure

### Phase 4+
- **Team**: 1-2 engineers (depends on scope)
- **Infrastructure**: Specialized tools (compression, binary codec)
- **Timeline**: 4+ weeks

---

## Release & Deployment Strategy

### Version Numbering
- Phase 1: v0.1.0 - Foundation release
- Phase 2: v0.2.0 - Query bridge release
- Phase 3: v1.0.0 - WebSocket, production-ready
- Phase 4: v1.1.0 - Optimization release

### Backward Compatibility
- Phase 1 → 2: API additions (no breaking changes)
- Phase 2 → 3: HTTP endpoint kept, WebSocket preferred
- Phase 3 → 4: All existing APIs preserved

### Rollout Strategy
- **Phase 1**: Direct release (foundational)
- **Phase 2**: Direct release (backward compatible)
- **Phase 3**: **Canary → Gradual ramp** (feature flag)
  - 1% traffic for 1 day
  - Ramp: 1% per day to 100%
  - Kill switch available
- **Phase 4+**: Direct release (performance improvements, no breaking changes)

---

## Success Metrics by Phase

### Phase 1 ✅
- All CRUD operations working
- Database integrity maintained
- API contract stable

### Phase 2 (Current)
- Queries transmitted successfully
- Zero message loss
- Latency <500ms
- All tests passing

### Phase 3 (Target)
- Query latency: p99 <100ms (target <50ms)
- 100k concurrent connections
- Error rate <0.1%
- 95%+ client adoption
- Zero message loss
- Production incident-free for 2 weeks

### Phase 4 (Target)
- Query latency: p99 <20ms (if needed)
- 1M+ concurrent connections (if needed)
- CPU/memory efficiency improved by 30%+

---

## Known Issues & Limitations

### Current (Phase 2)
- Query latency: 100-500ms (polling-based)
- Multiple TCP connections per session
- No native backpressure mechanism
- HTTP endpoint overhead

### Phase 3 Improvements
- ✅ Query latency: <50ms (WebSocket)
- ✅ Single connection per session
- ✅ Built-in backpressure
- ✅ Unified protocol

### Future Considerations
- Data migration strategies (if schema changes)
- Client library versioning
- Protocol versioning strategy

---

## Contact & Governance

**Project Owner**: See repository README  
**Phase 3 Technical Lead**: TBD  
**Architecture Review**: TBD  

**Decision Making**:
- Phase decisions: Team consensus
- Rollout decisions: Feature flag + metrics review
- Risk mitigation: Escalate to tech lead

---

## Appendix: Reference Documents

1. [Phase 3 Planning](specs/phase3_websocket_planning.md) - Detailed WebSocket design
2. [Integration Summary](INTEGRATION_SUMMARY.md) - Phase 1-2 status
3. [Architecture](specs/architecture.md) - System overview
4. [Streaming](specs/streaming.md) - LLM streaming details
5. [Quick Start](QUICK_START_QUERY_BRIDGE.md) - Getting started guide

---

## Changelog

### 2025-01-15
- Created Phase 3 planning document
- Added WebSocket groundwork (`websocket.rs`)
- Created this roadmap
- Updated Phase 3 expected timeline

### 2025-01-01
- Phase 2 integration complete
- Initial INTEGRATION_SUMMARY.md
