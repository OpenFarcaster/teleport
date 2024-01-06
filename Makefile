DATABASE_URL = "sqlite:farcaster.db"
MIGRATIONS_DIR = "./lib/storage/migrations"

db-create:
	DATABASE_URL=$(DATABASE_URL) sqlx db create

db-migrate:
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run --source $(MIGRATIONS_DIR)

db-query-prepare:
	DATABASE_URL=$(DATABASE_URL) cargo sqlx prepare --workspace

.PHONY: db-create db-migrate db-query-prepare