# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

.PHONY: test test-formal test-formal-native test-formal-parity check-formal-evidence test-headers test-secrets test-deny
TARGET_DIR ?= target

test:
	python3 -I scripts/test.py

test-formal-native:
	FORMAL_OUT='$(abspath $(TARGET_DIR))/formal/models' cargo test --offline --locked --test formal_models
	python3 -I scripts/formal.py --native --target '$(abspath $(TARGET_DIR))/formal'

test-formal: test-formal-native
	python3 -I scripts/formal.py --target '$(abspath $(TARGET_DIR))/formal'

test-formal-parity: test-formal

check-formal-evidence:
	python3 -I scripts/formal.py --verify-evidence --target '$(abspath $(TARGET_DIR))/formal'

test-headers:
	python3 -I scripts/checks.py headers

test-secrets:
	python3 -I scripts/checks.py secrets

test-deny:
	python3 -I scripts/checks.py deny
	python3 -I scripts/test-checks.py
