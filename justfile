# Run development containers
[group("Dev")]
docker:
    @docker compose up -d

# Run the server in development mode
[group("Dev")]
dev: docker
    @cargo run

# Clean the cargo cache
[group("Dev")]
clean:
    @cargo clean

# Build the cargo project
[group("Build")]
build:
    @cargo build
