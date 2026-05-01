.DEFAULT_GOAL := help

BUILD_SCRIPT := ./scripts/build-image.sh
RUN_SCRIPT := ./scripts/run-qemu.sh
SMOKE_SCRIPT := ./scripts/run-qemu-smoke.sh
TIMER_PROOF_SCRIPT := ./scripts/run-qemu-timer-proof.sh
TEST_SCRIPT := ./scripts/run-qemu-tests.sh
FAULTS_SCRIPT := ./scripts/run-qemu-fault-tests.sh

.PHONY: help build run smoke timer test faults

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make build   - build the kernel image and bootable ISO' \
		'  make run     - run the kernel in an interactive QEMU window' \
		'  make smoke   - run the headless smoke boot check' \
		'  make timer   - run the headless timer proof boot check' \
		'  make test    - run the bootable kernel test harness' \
		'  make faults  - run all dedicated fault-test scenarios'

build:
	bash $(BUILD_SCRIPT)

run:
	bash $(RUN_SCRIPT)

smoke:
	@if [ -f "$(SMOKE_SCRIPT)" ]; then \
		bash $(SMOKE_SCRIPT); \
	else \
		printf '%s\n' 'Missing ./scripts/run-qemu-smoke.sh; use `make run` for the current build path.'; \
		exit 1; \
	fi

timer:
	@if [ -f "$(TIMER_PROOF_SCRIPT)" ]; then \
		bash $(TIMER_PROOF_SCRIPT); \
	else \
		printf '%s\n' 'Missing ./scripts/run-qemu-timer-proof.sh; no timer proof path exists yet.'; \
		exit 1; \
	fi

test:
	@if [ -f "$(TEST_SCRIPT)" ]; then \
		bash $(TEST_SCRIPT); \
	else \
		printf '%s\n' 'Missing ./scripts/run-qemu-tests.sh; no integration test path exists yet.'; \
		exit 1; \
	fi

faults:
	@if [ -f "$(FAULTS_SCRIPT)" ]; then \
		bash $(FAULTS_SCRIPT); \
	else \
		printf '%s\n' 'Missing ./scripts/run-qemu-fault-tests.sh; Task 7 has not landed yet.'; \
		exit 1; \
	fi
