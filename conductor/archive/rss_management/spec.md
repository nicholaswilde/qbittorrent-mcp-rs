# RSS Management Specification

## Goal
Enable the MCP server to manage RSS feeds and auto-download rules in qBittorrent.

## Features

### 1. Feed Management
- **Add Feed**: Add a new RSS feed.
    - Endpoint: `/api/v2/rss/addFeed`
    - Parameters: `url`, `path` (internal path for the feed).
- **Remove Item**: Remove an RSS feed or folder.
    - Endpoint: `/api/v2/rss/removeItem`
    - Parameters: `path`.
- **Get All Feeds**: Retrieve all RSS feeds and their items.
    - Endpoint: `/api/v2/rss/allFeeds`
    - Parameters: `withData` (bool).

### 2. Rule Management
- **Set Rule**: Create or update an auto-download rule.
    - Endpoint: `/api/v2/rss/setRule`
    - Parameters: `ruleName`, `ruleDef` (JSON string defining the rule).
- **Remove Rule**: Delete an auto-download rule.
    - Endpoint: `/api/v2/rss/removeRule`
    - Parameters: `ruleName`.
- **Get All Rules**: Retrieve all auto-download rules.
    - Endpoint: `/api/v2/rss/allRules`.

## MCP Tools
- `add_rss_feed(url: String, path: String)`
- `remove_rss_item(path: String)`
- `get_rss_feeds()`: Returns all feeds and their contents.
- `set_rss_rule(name: String, definition: String)`: `definition` is a JSON string.
- `get_rss_rules()`: Returns all rules.
