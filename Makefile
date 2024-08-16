DATABASE_URL = "sqlite:farcaster.db"
MIGRATIONS_DIR = "./lib/storage/migrations"

define install_package
  if ! command -v $(1) >/dev/null 2>&1; then \
    echo "installing $(1)..."; \
    $(2); \
  fi
endef

define install_rust
  if ! command -v rustc >/dev/null 2>&1; then \
    echo "installing rust..."; \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
  fi
endef

define install_protobuf
  if ! command -v protoc >/dev/null 2>&1; then \
    echo "installing protobufs compiler..."; \
    $(1); \
  fi
endef

define install_prerequisites
  case $$OSTYPE in \
    darwin*) \
      echo "detected macos"; \
      $(call install_package,brew,/bin/bash -c "$$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"); \
      $(call install_rust); \
      $(call install_protobuf,brew install protobuf); \
      ;; \
    linux*) \
      echo "detected linux"; \
      $(call install_rust); \
      $(call install_protobuf,sudo apt update && sudo apt install -y protobuf-compiler); \
      ;; \
    *) \
      echo "unsupported operating system"; \
      exit 1; \
      ;; \
  esac
endef

db-create:
	DATABASE_URL=$(DATABASE_URL) sqlx db create

db-migrate:
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run --source $(MIGRATIONS_DIR)

db-query-prepare:
	DATABASE_URL=$(DATABASE_URL) cargo sqlx prepare --workspace

install:
	@$(call install_prerequisites)
	@if ! command -v sqlx >/dev/null 2>&1; then \
		echo "installing sqlx cli..."; \
		. $$HOME/.cargo/env && cargo install sqlx-cli; \
	fi
	@echo "all prerequisites installed successfully!"

.PHONY: db-create db-migrate db-query-prepare install
