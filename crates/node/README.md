# js-compat-check (JavaScript Compatibility Checker)

js-compat-check is a JavaScript compatibility checking tool that analyzes your code to detect JavaScript features and helps you understand compatibility across different browsers and runtime environments.

## Features

- 🔍 Detect JavaScript features used in your code
- 📊 Generate detailed browser compatibility reports
- 🌐 Support for both local files and remote URLs
- 🛠️ Available as both CLI tool and Node.js package

## Usage

### 1. CLI Usage

```bash
# npm
npx js-compat-check <file-or-url>
```

### 2. JavaScript API

```javascript
import { jsco } from 'js-compat-check';

// Check a local file
const report = await jsco('./path/to/file.js');

// Check from URL
const report = await jsco('https://example.com/script.js');
```