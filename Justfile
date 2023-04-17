deploy:
    scripts/deploy.sh

entity: (migrate "fresh")
    sea-orm-cli generate entity --lib --date-time-crate time --output-dir crates/entity/src

migrate arg="up":
    sea-orm-cli migrate {{arg}} -d crates/migration

rundb:
    docker run --rm -p 5432:5432/tcp -e POSTGRES_USER -e POSTGRES_PASSWORD postgres:15-alpine