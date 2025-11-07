#!/bin/bash

# Script to install git hooks for the wassette repository
# This script copies hooks from .git-hooks/ to .git/hooks/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GIT_HOOKS_DIR="$REPO_ROOT/.git-hooks"
GIT_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks..."

# Check if .git-hooks directory exists
if [ ! -d "$GIT_HOOKS_DIR" ]; then
    echo "Error: .git-hooks directory not found at $GIT_HOOKS_DIR"
    exit 1
fi

# Check if .git/hooks directory exists
if [ ! -d "$GIT_DIR" ]; then
    echo "Error: .git/hooks directory not found at $GIT_DIR"
    echo "Are you running this from the repository root?"
    exit 1
fi

# Copy all hooks from .git-hooks to .git/hooks (excluding README.md and other documentation files)
for hook in "$GIT_HOOKS_DIR"/*; do
    if [ -f "$hook" ]; then
        hook_name=$(basename "$hook")
        # Skip README and other documentation files
        if [[ "$hook_name" == "README.md" ]] || [[ "$hook_name" == *.md ]]; then
            continue
        fi
        echo "  Installing $hook_name..."
        cp "$hook" "$GIT_DIR/$hook_name"
        chmod +x "$GIT_DIR/$hook_name"
    fi
done

echo ""
echo "✓ Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
# List installed hooks (excluding documentation files)
for hook in "$GIT_HOOKS_DIR"/*; do
    if [ -f "$hook" ]; then
        hook_name=$(basename "$hook")
        if [[ "$hook_name" != "README.md" ]] && [[ "$hook_name" != *.md ]]; then
            echo "  - $hook_name"
        fi
    fi
done
echo ""
echo "These hooks will run automatically on git operations."
