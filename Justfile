deploy:
    scripts/deploy.sh

genentity: (migrate "fresh")
    sea-orm-cli generate entity --lib --date-time-crate time --output-dir entity/src

migrate arg="up":
    sea-orm-cli migrate {{arg}}

rundb:
    docker run --rm -p 5432:5432/tcp -e POSTGRES_USER -e POSTGRES_PASSWORD postgres:15-alpine