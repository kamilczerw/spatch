
# Default: show help
default:
    @just --list

# Run all tests using cargo nextest
test test="":
    cargo nextest run {{test}}

# Run tests with cargo nextest and watch for changes
test-watch test="":
  cargo watch -x "nextest run {{test}}"

coverage:
  cargo llvm-cov nextest --open

