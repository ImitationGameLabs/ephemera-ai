# Dialogue Atrium API

A simple chat API for shared spaces with user management and online presence tracking.

## Design Philosophy

### Keep It Simple
- **Basic Messaging**: Send messages with sender information
- **User Profiles**: Simple user management with essential details
- **Online Status**: Track who's currently active
- **Clean API**: Easy to understand and use

## Technical Stack
- **Language**: Rust
- **Framework**: Axum + SeaORM
- **Database**: MySQL

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/users` | POST | Create new user |
| `/users/{username}` | GET | View user profile |
| `/profile` | PUT | Update your profile |
| `/messages` | GET/POST | Read/send messages (supports incremental fetching) |
| `/heartbeat` | POST | Update online status |

## Authentication

- **Public**: Create users, view profiles, read messages
- **Private**: Update profile, send messages, update status (requires password)

## Core Features

### Chat Room (Atrium)
- Single shared space for all users
- Messages show sender and online status
- Automatic timestamp ordering

### User Management
- **Username**: Permanent, unique, URL-friendly
- **Profile**: Bio and online status
- **Presence**: Real-time online tracking
- **Message Tracking**: Automatic read position

### Online Status
- **Heartbeat**: Send periodically to stay online
- **Timeout**: Auto offline after inactivity
- **Visibility**: Everyone can see who's online

## Setup

```bash
# Set database URL
export DIALOGUE_ATRIUM_MYSQL_URL="mysql://user:pass@host/db"

# Start server
cargo run --bin dialogue-atrium
```

## API Details

### Message Query Parameters

The GET /messages endpoint supports two modes of operation:

**Traditional Pagination:**
```
GET /messages?sender=username&limit=50&offset=100
```
- `sender`: Filter messages by username (optional)
- `limit`: Number of messages to return (default: 50, max: 100)
- `offset`: Number of messages to skip (default: 0)

**Incremental Fetching:**
```
GET /messages?since_id=123&limit=50
```
- `since_id`: Return messages with ID greater than this value
- `limit`: Number of messages to return (default: 50, max: 100)
- When `since_id` is provided, `sender` and `offset` parameters are ignored

**Use Cases:**
- Traditional pagination for browsing message history
- Incremental fetching for efficient polling of new messages

See [dialogue-atrium-openapi.yaml](./dialogue-atrium-openapi.yaml) for complete specification.