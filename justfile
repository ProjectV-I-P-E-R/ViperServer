# Run development containers
[group("Dev")]
docker:
    @docker compose up -d

# Run the server in development mode
[group("Dev")]
dev:
    @cargo run