# OTL Makefile
#
# Usage:
#   make help      - Show available commands
#   make dev       - Start development environment
#   make build     - Build all crates
#   make test      - Run all tests
#
# Author: hephaex@gmail.com

.PHONY: help dev build test lint fmt clean docker-up docker-down docker-logs install

# Default target
.DEFAULT_GOAL := help

# Colors
GREEN  := \033[0;32m
YELLOW := \033[1;33m
NC     := \033[0m

# =============================================================================
# Help
# =============================================================================
help:
	@echo "OTL Development Commands"
	@echo "========================"
	@echo ""
	@echo "$(GREEN)Development:$(NC)"
	@echo "  make dev         - Start development environment (docker + build)"
	@echo "  make build       - Build all crates"
	@echo "  make test        - Run all tests"
	@echo "  make lint        - Run clippy linter"
	@echo "  make fmt         - Format code"
	@echo "  make check       - Run fmt check + clippy"
	@echo ""
	@echo "$(GREEN)Docker:$(NC)"
	@echo "  make docker-up   - Start all Docker services"
	@echo "  make docker-down - Stop all Docker services"
	@echo "  make docker-logs - View Docker logs"
	@echo "  make docker-gpu  - Start with GPU support"
	@echo ""
	@echo "$(GREEN)Database:$(NC)"
	@echo "  make db-reset    - Reset all databases (WARNING: deletes data)"
	@echo "  make db-status   - Check database status"
	@echo ""
	@echo "$(GREEN)CLI:$(NC)"
	@echo "  make cli-extract - Run extraction demo"
	@echo "  make cli-query   - Run query demo"
	@echo ""
	@echo "$(GREEN)Utilities:$(NC)"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make install     - Install OTL CLI globally"

# =============================================================================
# Development
# =============================================================================
dev: docker-up build
	@echo "$(GREEN)Development environment ready!$(NC)"
	@echo "Run: cargo run -p otl-cli -- --help"

build:
	@echo "Building all crates..."
	cargo build --workspace

build-release:
	@echo "Building release..."
	cargo build --workspace --release

test:
	@echo "Running tests..."
	cargo test --workspace

test-verbose:
	@echo "Running tests (verbose)..."
	cargo test --workspace -- --nocapture

lint:
	@echo "Running clippy..."
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	@echo "Formatting code..."
	cargo fmt --all

fmt-check:
	@echo "Checking format..."
	cargo fmt --all -- --check

check: fmt-check lint
	@echo "$(GREEN)All checks passed!$(NC)"

# =============================================================================
# Docker
# =============================================================================
docker-up:
	@echo "Starting Docker services..."
	docker compose up -d
	@echo "Waiting for services to be ready..."
	@sleep 5
	docker compose ps

docker-down:
	@echo "Stopping Docker services..."
	docker compose down

docker-logs:
	docker compose logs -f

docker-gpu:
	@echo "Starting Docker services with GPU support..."
	docker compose -f docker-compose.yml -f docker-compose.gpu.yml up -d

docker-status:
	docker compose ps

# =============================================================================
# Database
# =============================================================================
db-reset:
	@echo "$(YELLOW)WARNING: This will delete all data!$(NC)"
	@read -p "Are you sure? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1
	docker compose down -v
	docker compose up -d
	@echo "Databases reset."

db-status:
	@echo "Checking database status..."
	@echo ""
	@echo "PostgreSQL:"
	@docker compose exec -T postgres pg_isready -U otl || echo "Not ready"
	@echo ""
	@echo "SurrealDB:"
	@docker compose exec -T surrealdb /surreal isready || echo "Not ready"
	@echo ""
	@echo "Qdrant:"
	@curl -s http://localhost:6333/collections | jq . || echo "Not ready"

# =============================================================================
# CLI
# =============================================================================
cli-extract:
	@echo "Running extraction demo..."
	cargo run -p otl-cli -- extract "연차휴가는 최대 15일까지 사용할 수 있습니다. 병가 신청에는 진단서가 필요합니다."

cli-query:
	@echo "Running query demo (requires LLM)..."
	cargo run -p otl-cli -- query "연차휴가 사용 일수는?"

cli-verify:
	cargo run -p otl-cli -- verify demo
	cargo run -p otl-cli -- verify stats

# =============================================================================
# Ollama
# =============================================================================
ollama-pull:
	@echo "Pulling Ollama models..."
	docker compose exec ollama ollama pull llama2
	docker compose exec ollama ollama pull nomic-embed-text

ollama-list:
	docker compose exec ollama ollama list

# =============================================================================
# Utilities
# =============================================================================
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf target/

install:
	@echo "Installing OTL CLI..."
	cargo install --path crates/otl-cli

# Documentation
docs:
	@echo "Generating documentation..."
	cargo doc --workspace --no-deps --open

# Watch mode (requires cargo-watch)
watch:
	cargo watch -x "build --workspace"

watch-test:
	cargo watch -x "test --workspace"

# =============================================================================
# Setup (first time)
# =============================================================================
setup-ubuntu:
	@echo "Running Ubuntu setup..."
	chmod +x scripts/setup-ubuntu.sh
	./scripts/setup-ubuntu.sh

setup-env:
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "Created .env file. Please edit and set your API keys."; \
	else \
		echo ".env already exists."; \
	fi
