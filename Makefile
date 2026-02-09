# PeaPod Makefile — common build, test, install, and lint commands.
# Usage: make help

CARGO        ?= cargo
PREFIX       ?= /usr/local
BIN_DIR      := $(PREFIX)/bin
SERVICE_DIR  := $(HOME)/.config/systemd/user

.PHONY: help build test lint fmt clippy audit clean install uninstall service release dev run

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build all workspace crates (debug)
	$(CARGO) build -p pea-core -p pea-linux

release: ## Build pea-linux in release mode
	$(CARGO) build -p pea-linux --release

test: ## Run all tests
	$(CARGO) test -p pea-core --verbose

lint: fmt clippy ## Run all linters (fmt + clippy)

fmt: ## Check formatting
	$(CARGO) fmt --all -- --check

clippy: ## Run clippy with -D warnings
	$(CARGO) clippy -p pea-core -p pea-linux -- -D warnings

audit: ## Run cargo-audit for dependency vulnerabilities
	$(CARGO) audit

clean: ## Remove build artifacts
	$(CARGO) clean

dev: build test lint ## Build, test, and lint (quick verification)

run: build ## Build and run pea-linux (debug)
	$(CARGO) run -p pea-linux

install: release ## Install pea-linux binary and systemd service
	@echo "Installing pea-linux to $(BIN_DIR)..."
	install -Dm755 target/release/pea-linux $(BIN_DIR)/pea-linux
	@echo "Installed: $(BIN_DIR)/pea-linux"
	@mkdir -p $(HOME)/.config/peapod
	@echo "Config directory: $(HOME)/.config/peapod/"
	@echo ""
	@echo "To set up the systemd service, run:  make service"

service: ## Install and enable systemd user service
	@mkdir -p $(SERVICE_DIR)
	@sed 's|ExecStart=.*|ExecStart=$(BIN_DIR)/pea-linux|' pea-linux/misc/peapod.service > $(SERVICE_DIR)/peapod.service
	systemctl --user daemon-reload
	systemctl --user enable peapod.service
	@echo "Service installed. Start with: systemctl --user start peapod"

uninstall: ## Remove pea-linux binary and systemd service
	-systemctl --user stop peapod.service 2>/dev/null
	-systemctl --user disable peapod.service 2>/dev/null
	rm -f $(SERVICE_DIR)/peapod.service
	-systemctl --user daemon-reload 2>/dev/null
	rm -f $(BIN_DIR)/pea-linux
	@echo "PeaPod uninstalled. Config at ~/.config/peapod/ preserved."
