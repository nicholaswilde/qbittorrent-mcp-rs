# Search Plugin Management Specification

## Goal
Enable the MCP server to manage search plugins for qBittorrent, allowing the agent to install, update, and configure search sources.

## Features

### 1. Install Plugin
- **Functionality**: Install a new search plugin from a URL.
- **API Endpoint**: `/api/v2/search/installPlugin`
- **Parameters**: `sources` (URL).

### 2. Uninstall Plugin
- **Functionality**: Remove an existing search plugin.
- **API Endpoint**: `/api/v2/search/uninstallPlugin`
- **Parameters**: `names` (Plugin name).

### 3. Enable/Disable Plugin
- **Functionality**: Toggle a plugin's active status.
- **API Endpoint**: `/api/v2/search/enablePlugin`
- **Parameters**: `names` (Plugin name), `enable` (bool).

### 4. Update Plugins
- **Functionality**: Check for updates for all installed plugins.
- **API Endpoint**: `/api/v2/search/updatePlugins`

### 5. List Plugins
- **Functionality**: Retrieve a list of installed search plugins.
- **API Endpoint**: `/api/v2/search/plugins`

## MCP Tools
- `install_search_plugin(url: String)`: Install a plugin from a URL.
- `uninstall_search_plugin(name: String)`: Uninstall a plugin by name.
- `enable_search_plugin(name: String, enable: bool)`: Enable or disable a plugin.
- `update_search_plugins()`: Update all plugins.
- `get_search_plugins()`: List installed plugins.
