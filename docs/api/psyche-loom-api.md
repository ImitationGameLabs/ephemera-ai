# Psyche Loom API

A memory management API for storing, searching, and retrieving text-based memory fragments with metadata support.

## Design Philosophy

### Simple Memory Management
- **Store and Retrieve**: Basic CRUD operations for memory fragments
- **Metadata Support**: Flexible JSON metadata for rich context
- **Time-based Search**: Filter memories by creation time
- **Vector Search**: Advanced semantic search capabilities

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Service health check |
| `/api/v1/memory` | POST | Create new memory fragment |
| `/api/v1/memory` | GET | Search memory fragments |
| `/api/v1/memory/{id}` | GET | Retrieve specific memory |
| `/api/v1/memory/{id}` | DELETE | Delete memory fragment |

## Authentication

Currently **unauthenticated** - all endpoints are publicly accessible. Authentication can be added in future versions.

## Core Features

### Memory Fragments
- **Content**: Main text content of the memory
- **Metadata**: Optional JSON for additional context
- **Source**: Optional identifier for memory origin
- **Timestamps**: Automatic creation and update tracking
- **Unique IDs**: System-assigned identifiers

### Search Capabilities
- **Keyword Search**: Full-text search across memory content
- **Time Range**: Filter by creation/update time
- **Semantic Search**: Advanced similarity matching
- **Metadata Filtering**: Search within JSON metadata fields

## Usage Examples

### Creating a Memory
```bash
curl -X POST http://localhost:8080/api/v1/memory \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Meeting notes about project architecture",
    "metadata": {
      "type": "meeting",
      "project": "loom",
      "participants": ["alice", "bob"]
    },
    "source": "user_input"
  }'
```

### Searching Memories
```bash
# Basic keyword search
curl "http://localhost:8080/api/v1/memory?keywords=architecture"

# Search with time range
curl "http://localhost:8080/api/v1/memory?keywords=meeting&start_time=1640995200&end_time=1641081600"
```

## OpenAPI Specification

For complete API documentation including all request/response schemas, error codes, and detailed examples, see the [OpenAPI specification](./psyche-loom-openapi.yaml).
