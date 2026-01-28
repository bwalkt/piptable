# Git Hooks

This directory contains git hooks for the piptable project.

## Setup

Run the setup script to install the hooks:

```bash
./scripts/setup-hooks.sh
```

Or manually copy the hooks:

```bash
cp .githooks/pre-push .git/hooks/pre-push
chmod +x .git/hooks/pre-push
```

## Pre-push Hook

The pre-push hook runs before pushing commits to the remote repository and checks:

1. **Code Formatting** - Ensures all code is properly formatted with `cargo fmt`
2. **Linting** - Runs `cargo clippy` with warnings as errors
3. **Compilation** - Verifies the code compiles without errors

These are the same checks that run in CI, helping catch issues before they fail the build.

## Bypassing Hooks

In emergency situations, you can bypass the hooks using:

```bash
git push --no-verify
```

**Note:** Use this sparingly as it may cause CI failures.

## Troubleshooting

If the pre-push hook is not running:

1. Check that it's executable: `ls -la .git/hooks/pre-push`
2. Re-run the setup script: `./scripts/setup-hooks.sh`
3. Manually check the hook is installed: `cat .git/hooks/pre-push`