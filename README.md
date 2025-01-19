# JSCO (JavaScript Compatibility Checker)

JSCO is a JavaScript compatibility checking tool that analyzes your code to detect JavaScript features and helps you understand compatibility across different browsers and runtime environments.

## Features

- 🔍 Detect JavaScript features used in your code
- 📊 Generate detailed browser compatibility reports
- 🌐 Support for both local files and remote URLs
- 💾 Built-in caching system for better performance
- 🛠️ Available as both CLI tool and Node.js package

## Installation

### Using npm/pnpm

```bash
# Using npm
npm install jsco

# Using pnpm
pnpm add jsco
```

### From Source

1. Clone the repository
2. Install Rust (if not already installed)
3. Build the project:

```bash
cargo build --release
```

## Usage

### CLI

```bash
jsco <file-or-url>
```

### Node.js API

```javascript
import { jsco } from 'jsco';

// Analyze a local file
const report = await jsco('./path/to/file.js');

// Analyze from URL
const report = await jsco('https://example.com/script.js');
```

## Project Structure

- `crates/core` - Core functionality and analysis engine
- `crates/cli` - Command-line interface
- `crates/node` - Node.js bindings (using NAPI-RS)

## Requirements

- Node.js >= 10 (for Node.js package)
- Rust toolchain (for building from source)

## Development

```bash
# Build the project
pnpm build

# Run tests
pnpm test

# Build for all platforms
pnpm universal
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 