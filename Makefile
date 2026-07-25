#!/usr/bin/env bash
# Neo AGI OS — Makefile entry point
# Provides unified build targets across all languages.

.PHONY: help bootstrap build test lint clean format check

SCRIPTS := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))scripts

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

bootstrap: ## Install all toolchains and dependencies
	$(SCRIPTS)/bootstrap.sh

build: ## Build all components
	$(SCRIPTS)/build.sh

test: ## Run all tests
	$(SCRIPTS)/test.sh

lint: ## Run all linters
	$(SCRIPTS)/lint.sh

check: lint test ## Run lint and tests

format: ## Format all source code
	cargo fmt --all
	ruff format .
	pnpm format
	find . -name "*.go" -exec gofmt -w {} +

clean: ## Remove all build artifacts
	cargo clean
	rm -rf build/
	rm -rf node_modules/
	rm -rf .venv/
	rm -rf dist/
	rm -rf __pycache__
	rm -rf .pytest_cache
	rm -rf .mypy_cache
	find . -name "*.pyc" -delete
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true

tree: ## Show project structure
	@find . -not -path './.git/*' -not -path '*/node_modules/*' -not -path '*/target/*' -not -path '*/__pycache__/*' -not -name '.gitkeep' -not -name '*.pyc' | head -200

.PHONY: rust rust-test rust-clippy
rust: ## Build Rust workspace
	cargo build --workspace

rust-test: ## Test Rust workspace
	cargo test --workspace

rust-clippy: ## Lint Rust workspace
	cargo clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: cpp cpp-test
cpp: ## Build C++ components
	cmake -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build --parallel $$(nproc)

cpp-test: ## Test C++ components
	cmake -B build -DCMAKE_BUILD_TYPE=Debug -DNEO_ENABLE_TESTS=ON
	cmake --build build --parallel $$(nproc)
	cd build && ctest --output-on-failure

.PHONY: python python-test
python: ## Build Python packages
	pip install -e ./neural-network-framework/python -e ./sdk/python

python-test: ## Test Python packages
	python -m pytest

.PHONY: ts ts-test
ts: ## Build TypeScript packages
	pnpm -r build

ts-test: ## Test TypeScript packages
	pnpm test
