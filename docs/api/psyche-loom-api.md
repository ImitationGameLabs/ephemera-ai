# Psyche Loom API

A memory management API for storing, searching, and retrieving text-based memory fragments with metadata support.

## Design Philosophy

### View-Based Resource Design
- **RESTful Resources**: Memories are first-class resources under `/api/v1/memories`
- **Views**: Structured access patterns via `/views/*` endpoints
- **Metadata Support**: Flexible JSON metadata for rich context
- **Time-based Queries**: Timeline view for time range queries

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Service health check |
| `/api/v1/memories` | POST | Create new memory fragment(s) |
| `/api/v1/memories/{id}` | GET | Retrieve specific memory |
| `/api/v1/memories/{id}` | DELETE | Delete memory fragment |
| `/api/v1/memories/views/recent` | GET | Get recent memories |
| `/api/v1/memories/views/timeline` | GET | Get memories in time range |

## Authentication

Currently **unauthenticated** - all endpoints are publicly accessible. Authentication can be added in future versions.

## Core Features

### Memory Fragments
- **Content**: Main text content of the memory
- **Metadata**: Optional JSON for additional context
- **Source**: Optional identifier for memory origin
- **Timestamps**: Automatic creation and update tracking
- **Unique IDs**: System-assigned identifiers

### View Endpoints
- **Recent View**: Get the most recent N memories
- **Timeline View**: Query memories within a time range using ISO 8601 format

## Usage Examples

### Creating a Memory
```bash
curl -X POST http://localhost:8080/api/v1/memories \
  -H "Content-Type: application/json" \
  -d '{
    "fragments": [{
      "content": "Meeting notes about project architecture",
      "source": {"channel": "meeting", "identifier": "user_input"}
    }]
  }'
```

### Getting Recent Memories
```bash
curl "http://localhost:8080/api/v1/memories/views/recent?limit=10"
```

### Timeline Query (Time Range)
```bash
# Using ISO 8601 time format
curl "http://localhost:8080/api/v1/memories/views/timeline?from=2024-01-01T00:00:00Z&to=2024-12-31T23:59:59Z"

# With pagination
curl "http://localhost:8080/api/v1/memories/views/timeline?from=2024-01-01T00:00:00Z&to=2024-12-31T23:59:59Z&limit=50&offset=0"
```

## OpenAPI Specification

For complete API documentation including all request/response schemas, error codes, and detailed examples, see the [OpenAPI specification](./psyche-loom-openapi.yaml).
