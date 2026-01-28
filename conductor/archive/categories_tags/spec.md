# Categories & Tags Specification

## Goal
Enable the MCP server to organize torrents using categories and tags.

## Features

### 1. Categories
- **Create Category**: Create a new category with a save path.
    - Endpoint: `/api/v2/torrents/createCategory`
- **Set Category**: Assign a category to one or more torrents.
    - Endpoint: `/api/v2/torrents/setCategory`
- **List Categories**: Retrieve all categories.
    - Endpoint: `/api/v2/torrents/categories`

### 2. Tags
- **Add Tags**: Add tags to torrents.
    - Endpoint: `/api/v2/torrents/addTags`
- **Remove Tags**: Remove tags from torrents.
    - Endpoint: `/api/v2/torrents/removeTags`
- **Create Tags**: Create new tags.
    - Endpoint: `/api/v2/torrents/createTags`

## MCP Tools
- `create_category(name: String, save_path: String)`
- `set_torrent_category(hashes: String, category: String)`
- `get_categories()`: Returns map of categories.
- `add_torrent_tags(hashes: String, tags: String)`: Tags are comma-separated.
