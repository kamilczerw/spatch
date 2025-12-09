
# Default: show help
default:
    @just --list

# Run all tests using cargo nextest
test:
    cargo nextest run

# Run tests with cargo nextest and watch for changes
test-watch:
  cargo watch -x "nextest run"

