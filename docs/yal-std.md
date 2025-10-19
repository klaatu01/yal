# YAL Lua Standard Library Reference

This document describes the Lua APIs exposed by the YAL host runtime.  
All modules are available via `package.preload` and must be required as `yal.*`.

---

## Table of Contents

- [UI Module (`yal.ui`)](#ui-module-yalui)
- [Base64 Module (`yal.base64`)](#base64-module-yalbase64)
- [Logging Module (`yal.log`)](#logging-module-yallog)
- [Database Module (`yal.db`)](#database-module-yaldb)
- [JSON Module (`yal.json`)](#json-module-yaljson)
- [Socket Module (`yal.socket`)](#socket-module-yalsocket)
- [HTTP Module (`yal.http`)](#http-module-yalhttp)
- [Vendor Searcher](#vendor-searcher)
- [Error Conventions](#error-conventions)

---

## UI Module (`yal.ui`)

### Overview
Provides functions for interacting with host-driven user interfaces such as forms and prompts.

### Functions

#### `prompt(form) -> any`
Sends a structured form definition to the host for display.  
The function blocks until the user submits a response.

**Parameters**
- `form` — Table defining the form structure (JSON-compatible).

**Returns**
- A Lua value representing the response from the user.

**Errors**
- Throws on transmission errors or malformed form data.

---

## Base64 Module (`yal.base64`)

### Overview
Provides encoding and decoding utilities for Base64 and URL-safe Base64 transformations.

### Functions

#### `encode(input [, opts]) -> string`
Encodes a Lua string to Base64.

#### `encode_url(input [, opts]) -> string`
Encodes to URL-safe Base64.

#### `decode(b64 [, opts]) -> string`
Decodes Base64 input into a raw byte string.

#### `decode_url(b64 [, opts]) -> string`
Decodes URL-safe Base64 input into a raw byte string.

**Options**
- `opts.pad` (boolean, default `true`) — Whether to include padding (`=`) characters.

**Errors**
- Throws if the input is not valid Base64.

---

## Logging Module (`yal.log`)

### Overview
Provides access to the host logging system.

### Functions

#### `debug(message)`
Logs a debug-level message.

#### `info(message)`
Logs an informational message.

#### `warn(message)`
Logs a warning message.

#### `error(message)`
Logs an error message.

**Parameters**
- `message` — String message to log.

**Return Value**
- None.

---

## Database Module (`yal.db`)

### Overview
Implements a persistent key–value storage system with automatic JSON serialization.

### Storage Resolution
1. `$XDG_STATE_HOME/yal/plugins/<namespace>.json`
2. `$XDG_CONFIG_HOME/yal/plugins/<namespace>.json`
3. `$HOME/.yal/plugins/<namespace>.json`
4. `./plugins/<namespace>.json` (fallback)

### Functions

#### `open(namespace) -> kv`
Opens or creates a key–value database under the specified namespace.

### KV Methods

| Method | Description |
|---------|-------------|
| `get(key)` | Returns the stored value for `key` or `nil` if absent. |
| `set(key, value)` | Writes a value and persists the change. |
| `set_many(table)` | Writes multiple key–value pairs at once. |
| `delete(key)` | Removes a key from the store. |
| `keys()` | Returns an array of all stored keys. |
| `all()` | Returns a table of all stored entries. |
| `path()` | Returns the file path to the storage file. |
| `flush()` | Forces a full write to disk. |
| `reload()` | Reloads the database from disk. |

**Errors**
- Throws on file I/O or JSON serialization errors.

---

## JSON Module (`yal.json`)

### Overview
Provides JSON serialization and deserialization.

### Functions

#### `encode(value) -> string`
Converts a Lua value to a JSON string.

#### `decode(string) -> value`
Parses a JSON string into a Lua value.

**Errors**
- Throws on invalid input or serialization errors.

---

## Socket Module (`yal.socket`)

### Overview
Provides asynchronous TCP server and client functionality.

### Server API

#### `bind(ip, port) -> server`
Creates a TCP listener bound to the given address and port.

#### `server:settimeout(seconds | nil)`
Sets or clears an accept timeout.

#### `server:accept() -> client | nil, "timeout"`
Waits for an incoming connection.

#### `server:close()`
Closes the server socket.

### Client API

#### `client:settimeout(seconds | nil)`
Sets or clears read/write timeouts.

#### `client:receive(mode) -> string | nil, "timeout"`
Reads data from the connection.  
Modes:
- `"*l"` — Line mode (newline-stripped).
- `"*a"` — Read until EOF (max 16 MiB).
- `<number>` — Read a fixed number of bytes.

#### `client:send(data) -> integer | nil, "timeout"`
Sends data to the connection.

#### `client:close()`
Closes the client connection.

**Timeout Behavior**
- Returns `nil, "timeout"` on timeout.
- Throws on other I/O errors.

---

## HTTP Module (`yal.http`)

### Overview
Provides a concurrent, asynchronous HTTP client with configurable limits and request parameters.

### Default Limits
| Parameter | Default |
|------------|----------|
| `max_concurrent` | 16 |
| `timeout_ms` | 10 000 |
| `max_body_bytes` | 4 MiB |
| `max_redirects` | 5 |

### Functions

#### `request(opts) -> response`
Performs a configurable HTTP request.

**Options**
| Field | Type | Description |
|--------|------|-------------|
| `method` | string | HTTP method (default `"GET"`). |
| `url` | string | Target URL (required). |
| `headers` | table | Custom headers. |
| `query` | table | URL query parameters. |
| `timeout_ms` | integer | Request timeout in milliseconds. |
| `max_body_bytes` | integer | Maximum response size in bytes. |
| `max_redirects` | integer | Maximum redirects allowed. |
| `body` | string | Raw text body. |
| `body_bytes` | table | Raw bytes array. |
| `json` | table | JSON-encoded body. |
| `save_to` | string | Path to save response body. |

#### `get(url [, opts]) -> response`
Performs a GET request.

#### `post_json(url, body [, opts]) -> response`
Performs a POST request with a JSON body.

#### `set_default_header(name, value)`
Defines a global default header for all subsequent requests.

### Response Table
| Field | Type | Description |
|--------|------|-------------|
| `status` | integer | HTTP status code. |
| `headers` | table | Response headers. |
| `body` | string or nil | Response body. |

**Errors**
- Throws on invalid parameters, request failures, or exceeded body limits.

---

## Vendor Searcher

### Overview
Adds a searcher to `package.searchers` for loading modules from a `vendor` directory.

### Resolution Rules
For `require("a.b.c")`, the loader attempts the following paths:
1. `vendor/a/b/c.lua`
2. `vendor/a/b/c/init.lua`

If not found, a descriptive error message is appended to the module search failure chain.

---

## Error Conventions

| Type | Behavior |
|------|-----------|
| Parameter/logic errors | Raise Lua error |
| I/O or network failures | Raise Lua error |
| Timeouts (socket) | Return `nil, "timeout"` |
| Serialization errors | Raise Lua error |

---

© 2025 YAL Runtime — All rights reserved.
