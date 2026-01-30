#!/bin/bash
# Script to set up git hooks for the project

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$PROJECT_ROOT/.githooks"
GIT_HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

echo "🔧 Setting up git hooks for piptable..."

# Create hooks directory if it doesn't exist
mkdir -p "$GIT_HOOKS_DIR"

# Copy pre-push hook
if [ -f "$HOOKS_DIR/pre-push" ]; then
    cp "$HOOKS_DIR/pre-push" "$GIT_HOOKS_DIR/pre-push"
    chmod +x "$GIT_HOOKS_DIR/pre-push"
    echo "✅ Installed pre-push hook"
else
    echo "⚠️  Warning: pre-push hook template not found at $HOOKS_DIR/pre-push"
fi

echo ""
echo "✨ Git hooks setup complete!"
echo ""
echo "The pre-push hook will run the following checks before pushing:"
echo "  • Code formatting (cargo fmt)"
echo "  • Linting (scripts/clippy.sh, defaults to PYO3_PYTHON=python3.13)"
echo "  • Compilation check (cargo check)"
echo ""
echo "To bypass hooks in an emergency, use: git push --no-verify"
