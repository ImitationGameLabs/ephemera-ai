# EPHA-Agent Documentation

## Quick Start

**New to EPHA-Agent?** Start here:
- 🎯 [Executive Summary](architecture-executive-summary.md) - 2 min read
- 🏗️ [Core Architecture](architecture-core.md) - 5 min read
- 💻 [Developer Quickstart](architecture-developer-quickstart.md) - 10 min read

## 🔧 Technical Deep Dives
| Document | Focus | When to Read |
|----------|-------|--------------|
| [Phase Architecture](architecture-phases.md) | Dynamic Phase System, Custom Phases | When understanding the core agent architecture |
| [Observability & Control API](architecture-observability.md) | Monitoring, Metrics, Dynamic Control | When implementing agent control or monitoring |
| [Complete Architecture Design](architecture-design.md) | Comprehensive Technical Details | For detailed reference (1800+ lines) |

## Key Concepts

### 🔄 Infinite Loop Architecture
EPHA agents run continuously, processing events as part of their natural cognitive flow rather than being interrupted by user input.

### 🧩 Dynamic Phase System
EPHA agents use pluggable, configurable phases instead of hardcoded loops, allowing custom cognitive architectures.

**Learn more**: [Phase Architecture](architecture-phases.md)

### Comprehensive Observability & Window-Based Rate Control
Per-phase performance tracking, LLM request monitoring, and real-time metrics.
Agents are limited to `N` loops per `X` seconds window, providing flexible resource management.

**Learn more**: [Observability & Control](architecture-observability.md)

### Dynamic Configuration
Runtime adjustment of agent behavior, rate limits, and resource constraints without restart.

**Learn more**: [Observability & Control](architecture-observability.md)

## Quick Examples

### Create a Controlled Agent
```rust
let agent = MyAgent::new().await;
agent.run().await?;
```

### Configure Rate Limiting
```bash
curl -X PUT http://localhost:8080/api/agents/agent-001/rate \
  -d '{"window_seconds": 10, "max_loops_per_window": 50}'
```

### Monitor Performance
```rust
let metrics = controller.get_agent_metrics("agent-001").await?;
println!("Current performance: {:?}", metrics);
```

## Implementation Phases

1. **Foundation** - Core loop and event system
2. **Communication** - Async I/O and routing
3. **Intelligence** - Decision making and learning
4. **Scale** - Multi-node deployment
5. **Security** - Behavioral controls
6. **Production** - Hardening and deployment

**Details**: [Implementation Timeline](architecture-executive-summary.md#implementation-timeline)

## Getting Help

### 📚 Documentation
- Choose the right document for your role (see table above)
- Search within documents for specific topics
- Check code examples in the Developer Quickstart

### 🔍 Common Questions
- **What makes EPHA different?** - See [Executive Summary](architecture-executive-summary.md#key-differentiators)
- **How does the infinite loop work?** - See [Core Architecture](architecture-core.md#key-innovation-infinite-loop-architecture)

### 🛠️ Development Resources
- [Developer Quickstart](architecture-developer-quickstart.md) - Complete implementation guide
- [Observability API](architecture-observability.md) - REST API documentation
- [Core Architecture](architecture-core.md) - Technical reference

---