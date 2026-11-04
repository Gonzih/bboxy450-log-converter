# Shared license gate copied from bboxy450-term/scripts/suite-licenses.py.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
.PHONY: help test test-licenses check-license-packages
help:
	@printf '%s\n' 'make test-licenses              Offline suite policy and negative controls' 'make check-license-packages    Check real Cargo package lists when present'
test: test-licenses
test-licenses:
	python3 -I scripts/suite-licenses.py
	python3 -I scripts/test-suite-licenses.py
check-license-packages: test-licenses
	python3 -I scripts/suite-licenses.py --packages
