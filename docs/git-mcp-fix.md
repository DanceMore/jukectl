# Fixing Git MCP Ownership Issue

## Problem
The git MCP server is encountering a "dubious ownership" error when trying to access the repository:

```
fatal: detected dubious ownership in repository at '/workspace'
To add an exception for this directory, call:
	git config --global --add safe.directory /workspace
```

## Root Cause
The git MCP server runs in a Docker container with mounted volumes. Git's security feature detects that the mounted directory has "dubious ownership" and prevents access to it.

## Solution Approaches

### Option 1: Configure Git in the Docker Image (Recommended)
Modify the git MCP Docker image to include proper Git configuration:

```bash
# In the Docker image startup script or configuration
git config --global --add safe.directory /workspace
```

### Option 2: Update MCP Configuration to Handle Git Security
Update the mcp.json configuration to properly handle Git security settings.

### Option 3: Use Docker Volume Mount with Proper Permissions
Ensure the mounted directory has proper ownership and permissions.

## Implementation Steps

1. **For development environments:**
   - Run the following command to trust the workspace directory:
   ```bash
   git config --global --add safe.directory /workspace
   ```

2. **For Docker container environments:**
   - Modify the git MCP Docker image to include the Git configuration as part of its startup process
   - Or configure the container to run with proper user permissions

3. **For MCP configuration:**
   - Ensure the mcp.json file properly configures the git operations

## Testing the Fix

After implementing the fix, verify that the git MCP is functional by running:

```bash
# Test git status operation
mcp git_status

# Test other git operations  
mcp git_log
mcp git_diff_unstaged
```

## Current Configuration

The current mcp.json configuration for the git MCP server:
```json
{
  "mcpServers": {
    "git": {
      "command": "docker",
      "args": [
        "run",
        "--rm",
        "-i",
        "--mount",
        "type=bind,src=/home/neoice/code/jukectl,dst=/local-directory",
        "mcp/git"
      ],
      "alwaysAllow": [
        "git_status",
        "git_diff_unstaged",
        "git_diff_staged",
        "git_diff",
        "git_commit",
        "git_add",
        "git_reset",
        "git_log",
        "git_create_branch",
        "git_checkout",
        "git_show"
      ]
    }
  }
}
```

## Note
The issue occurs because the Docker container runs with different user context than the host system, causing Git to flag the mounted repository as having "dubious ownership". The fix requires either configuring Git to trust the directory or ensuring proper user permissions in the container.