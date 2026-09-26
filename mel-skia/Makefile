# Convenience wrappers around the repo's documented flows (ADR 002 — the
# real work happens in scripts/run-sim.sh and scripts/run-device.py).
#
#   make run                         # latest example on the iOS Simulator
#   make run EXAMPLE=002_blank_app   # a specific example
#   make run-device                  # 006 on a signed, provisioned iPhone
#   make check                       # everything reviews require to be green

EXAMPLE ?= 008_app_lifecycle

.PHONY: run run-device check

run:
	scripts/run-sim.sh $(EXAMPLE)

run-device: EXAMPLE = 006_frame_timing
run-device:
	python3 scripts/run-device.py $(EXAMPLE)

# Host checks + iOS clippy (CONTEXT.md §6 / mel_spec.md §6).
check:
	python3 -B -m unittest discover -s scripts -p 'test_*.py'
	bash -n scripts/run-sim.sh
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	cargo test -p mel-platform --example 006_frame_timing
	cargo clippy --target aarch64-apple-ios-sim -p mel-platform --all-targets -- -D warnings
