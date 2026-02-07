# StandX Point

A Rust-based project implementing market making strategies with a fractal architecture approach.

## Structure

This project is organized as a Cargo workspace with the following members:

- **crates/standx-point-adapter**: Adapter layer implementation.
- **crates/standx-point-mm-strategy**: Market making strategy logic.
- **examples/json-persistence-demo**: Demonstration of persistence capabilities.

## Getting Started

### Prerequisites

- Rust (latest stable recommended)

### Build

```bash
cargo build
```

### Run Examples

To run the JSON persistence demo:

```bash
cargo run -p json-persistence-demo
```

## Development

Please refer to [AGENTS.md](./AGENTS.md) for detailed development guidelines, including the **Fractal Context Protocol**.
