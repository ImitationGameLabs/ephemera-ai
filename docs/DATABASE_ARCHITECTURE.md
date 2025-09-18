# Database Architecture

## Overview
Ephemera AI uses a hybrid database architecture combining MySQL for structured data storage and Qdrant for vector-based semantic search. This design provides the best of both worlds: relational database integrity for metadata and vector database efficiency for semantic similarity searches.

## Database Selection Rationale

### MySQL (Structured Data Storage)
**Why MySQL:**
- **Relational Integrity**: Strong support for complex queries, transactions, and data consistency
- **Mature Ecosystem**: Extensive tooling, monitoring, and backup solutions
- **Performance**: Excellent for structured data with proper indexing
- **Read-Optimized**: Well-suited for our read-heavy workload pattern
- **JSON Support**: Native JSON column support for flexible metadata storage

### Qdrant (Vector Database)
**Why Qdrant:**
- **Specialized Vector Search**: Optimized for high-performance similarity searches
- **Production Ready**: HTTP/REST API, scalability, and cloud-native features
- **Hybrid Search Support**: Combines vector search with metadata filtering
- **Open Source**: Apache 2.0 license with commercial support options
- **Rust Native**: Excellent integration with our Rust codebase

## Architecture Design

### Data Flow
```
+----------------+     +----------------+     +----------------+
|   MySQL        |     |   Application  |     |   Qdrant       |
|   (Metadata)   |<--->|   Logic        |<--->|   (Vectors)    |
+----------------+     +----------------+     +----------------+
      |                       |                       |
      | Structured Queries    | Semantic Queries      | Vector Operations
      | (Time, Filters)       | (Hybrid Search)       | (Similarity Search)
```

### MySQL Schema Design

#### Memory Fragments Table
```sql
CREATE TABLE memory_fragments (
    id BIGINT PRIMARY KEY,               -- Millisecond timestamp as unique ID
    content TEXT NOT NULL,               -- Memory content text
    created_at BIGINT NOT NULL,          -- Unix timestamp
    source VARCHAR(50) NOT NULL,         -- Memory source type
    importance TINYINT UNSIGNED,         -- Subjective importance (0-255)
    confidence TINYINT UNSIGNED,         -- Confidence level (0-255)
    tags JSON,                           -- Categorization tags
    notes TEXT,                          -- Free-form notes
    associations JSON,                   -- Related memory IDs

    -- Indexes for efficient querying
    INDEX idx_created_at (created_at),
    INDEX idx_source (source),
    INDEX idx_importance (importance),
    INDEX idx_tags ((CAST(tags AS CHAR(255)))),
    FULLTEXT INDEX idx_content (content)
);
```

### Qdrant Configuration

#### Collection Setup
```yaml
collection_name: "ephemera_memory_vectors"
vectors:
  size: 384                    # Embedding dimension
  distance: Cosine            # Similarity metric
  hnsw_config:                # Approximate nearest neighbors
    m: 16
    ef_construct: 100
```

## Implementation Strategy

### Hybrid Search Workflow
1. **Semantic First**: Query Qdrant for vector similarity matches
2. **Metadata Enrichment**: Fetch complete records from MySQL using matched IDs
3. **Fallback to Keyword**: If semantic search yields few results, use MySQL full-text search
4. **Time Filtering**: Apply temporal constraints at MySQL level

### Data Consistency
- **Atomic Writes**: Both MySQL and Qdrant updates in transaction-like pattern
- **Eventual Consistency**: Qdrant updates may lag slightly behind MySQL
- **Recovery Mechanism**: Background job to sync mismatched data

## Performance Considerations

### Indexing Strategy
- **MySQL**: Composite indexes for common query patterns (time + source + importance)
- **Qdrant**: HNSW index for fast approximate nearest neighbor search
- **Embedding Cache**: LRU cache for frequently accessed text embeddings

### Query Optimization
- **Batch Operations**: Process multiple memories in single database calls
- **Connection Pooling**: Reuse database connections for reduced overhead
- **Selective Loading**: Only fetch necessary fields for each query type

## Migration Plan (From MeiliSearch)

### Phase 1: Dual Writing
- Write to both MeiliSearch and new MySQL+Qdrant system
- Compare query results for consistency
- Monitor performance impact

### Phase 2: Read Switching
- Gradually route read queries to new system
- Maintain fallback to MeiliSearch during transition
- Validate data integrity

### Phase 3: Full Migration
- Disable MeiliSearch writes
- Archive old MeiliSearch data
- Remove MeiliSearch dependencies

## Monitoring and Maintenance

### Key Metrics
- **MySQL**: Query latency, connection pool usage, cache hit rate
- **Qdrant**: Search latency, memory usage, vector index quality
- **Application**: End-to-end query performance, error rates

### Backup Strategy
- **MySQL**: Daily full backups + binary log replication
- **Qdrant**: Snapshot-based backups to cloud storage
- **Cross-Validation**: Periodic consistency checks between systems

## Future Considerations

### Scalability
- **MySQL Read Replicas**: For read-heavy workloads
- **Qdrant Sharding**: Horizontal scaling for large vector datasets
- **Embedding Service**: Dedicated microservice for embedding generation

### Enhanced Features
- **Graph Relationships**: Neo4j integration for complex memory associations
- **Real-time Updates**: WebSocket notifications for memory changes
- **Advanced Analytics**: Time-series analysis of memory patterns