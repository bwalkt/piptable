#!/usr/bin/env bash
set -euo pipefail

PYO3_PYTHON=${PYO3_PYTHON:-python3.13} \
  cargo clippy --workspace --all-targets "$@"
