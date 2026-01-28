# App Preferences Specification

## Goal
Enable the MCP server to inspect and modify qBittorrent's application preferences.

## Features

### 1. Get Preferences
- **Functionality**: Retrieve all application preferences.
- **API Endpoint**: `/api/v2/app/preferences`
- **Return Type**: A large JSON object containing all settings (save paths, network settings, scheduler, etc.).

### 2. Set Preferences
- **Functionality**: Update one or more application preferences.
- **API Endpoint**: `/api/v2/app/setPreferences`
- **Parameters**: `json` (A JSON object containing the settings to update).

## MCP Tools
- `get_app_preferences()`: Returns the full configuration of the qBittorrent instance.
- `set_app_preferences(preferences: String)`: Updates settings using a JSON string.
