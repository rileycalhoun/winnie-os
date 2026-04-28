.DEFAULT_GOAL := help

BUILD_SCRIPT := ./build.sh
SMOKE_SCRIPT := ./scripts/run-qemu-smoke.sh
TEST_SCRIPT := ./scripts/run-qemu-tests.sh
FAULTS_SCRIPT := ./scripts/run-qemu-fault-tests.sh

.PHONY: help build run smoke test faults

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make build   - compile the kernel only' \
		'  make run     - build the ISO and boot it in QEMU' \
		'  make smoke   - run the dedicated smoke script when it exists' \
		'  make test    - run the bootable kernel test harness in QEMU' \
		'  make faults  - run the dedicated fault-test script when it exists'

build:
	cargo build

run:
	$(BUILD_SCRIPT)

smoke:
	@if [ -x "$(SMOKE_SCRIPT)" ]; then \
		$(SMOKE_SCRIPT); \
	else \
		printf '%s\n' 'Missing ./scripts/run-qemu-smoke.sh; use `make run` for the current build path.'; \
		exit 1; \
	fi

test:
	@if [ -f "./src/main.rs" ]; then \
		cargo test --bin winnie-os; \
	elif [ -x "$(TEST_SCRIPT)" ]; then \
		$(TEST_SCRIPT); \
	else \
		printf '%s\n' 'Missing the bootable kernel test entrypoint and ./scripts/run-qemu-tests.sh; no integration test path exists yet.'; \
		exit 1; \
	fi

faults:
	@if [ -x "$(FAULTS_SCRIPT)" ]; then \
		$(FAULTS_SCRIPT); \
	else \
		printf '%s\n' 'Missing ./scripts/run-qemu-fault-tests.sh; Task 7 has not landed yet.'; \
		exit 1; \
	fi
