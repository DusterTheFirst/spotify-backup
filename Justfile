deploy:
    scripts/deploy.sh

rundb:
    docker build -t spotify-backup-database database/
    docker run --rm -p 8880:8080/tcp -e SURREAL_USER -e SURREAL_PASS spotify-backup-database