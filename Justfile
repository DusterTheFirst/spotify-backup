# just manual: https://github.com/casey/just#readme

_default:
  just --list

# Deploy a new release
deploy:
    scripts/deploy.sh

# Genereate the entities from the database
entity: (migrate "fresh")
    sea-orm-cli generate entity --lib --date-time-crate time --output-dir crates/entity/src

# Apply migrations to the database
migrate arg="up":
    sea-orm-cli migrate {{arg}} -d crates/migration

# Run the database
rundb:
    docker run --rm -p 5432:5432/tcp -e POSTGRES_USER -e POSTGRES_PASSWORD postgres:15-alpine