# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# md2html justfile orchestrator: every recipe lives in just/*.just.

import 'just/shared.just'
import 'just/help.just'
import 'just/test.just'
import 'just/build.just'
import 'just/workflow.just'
import 'just/dev.just'
import 'just/legal.just'
import 'just/security.just'
import 'just/analysis.just'
import 'just/analysis_ci.just'
import 'just/cache.just'
import 'just/codesign.just'
import 'just/local.just'

# Default recipe - show available commands.
default: _default-help

import 'just/adopt.just'
