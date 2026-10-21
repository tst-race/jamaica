.PHONY:all
all:
	@echo "usage: make <TARGET>"
	@exit 1


.PHONY: help
help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'


define source_pcaps
	find ../../data/remora-source-pcaps/ -type f -name '*.pcap'
endef

.PHONY: run
run:
	@$(call source_pcaps) | RUST_LOG="Debug" xargs -n1 -I{} cargo run --release -- {} --force --output ../../data/remora-datasets/

# Process any new PCAPs, that is PCAPs that don't have corresponding output directories in the dataset directory.
.PHONY: run_new
run_new:
	$(call source_pcaps) | RUST_LOG="Debug" xargs -n1 -I{} cargo run --release -- {} --headless --output ../../data/remora-datasets/

.PHONY: format
format:
	find src/ -type f -name '*rs' | xargs rustfmt

.PHONY: lint
lint:
	cargo clippy
