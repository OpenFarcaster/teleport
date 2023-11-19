DATABASE_URL = "sqlite:farcaster.db"
MIGRATIONS_DIR = "./lib/hub/migrations"

db-create:
	DATABASE_URL=$(DATABASE_URL) sqlx db create

db-migrate:
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run --source $(MIGRATIONS_DIR)

.PHONY: db-create db-migrate