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
| `/messages` | GET/POST | Read/send messages |
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

See [dialogue-atrium-openapi.yaml](./dialogue-atrium-openapi.yaml) for complete specification.