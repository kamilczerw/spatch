
# Default: show help
default:
    @just --list

# Run all tests using cargo nextest
test:
    cargo nextest run

# Run tests with cargo nextest and watch for changes
test-watch test="":
  cargo watch -x "nextest run {{test}}"

coverage:
  cargo llvm-cov nextest --open

