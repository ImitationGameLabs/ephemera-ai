# Memory Stream Simplification

## Background

Previously, the memory stream design included multiple structured fields:
- `importance` - numeric rating of memory significance
- `confidence` - certainty level
- `tags` - categorical labels
- Other metadata fields

These fields were intended for structured filtering and querying.

## The Problem

This was over-engineered:

- AI searches memory using concepts, not numeric conditions
- Precise numeric queries (e.g., "importance = 187") are rarely needed
- Added storage complexity and maintenance overhead
- Schema bloat without clear use cases

## The Decision

Simplified to minimal field set:
- `id` - unique identifier
- `content` - the actual memory content
- `timestamp` - when it occurred
- `source` - origin of the memory

Fields like `importance` and `tags` are stored as Qdrant payload for search boosting, not as independent filter conditions.

## Result

- MySQL schema simplified
- Storage efficiency improved
- Retrieval logic cleaner
- Less maintenance burden

## Status: Completed

The memory stream now follows the principle: keep the raw event log minimal. Any AI processing of memories (importance ratings, tags, associations) belongs elsewhere in the architecture.
