# GASP (Gee Another Schema Parser)

GASP is a high-performance Rust-based parser and validator for WAIL (Widely Applicable Interface Language) schemas and JSON responses. It's specifically designed to work with Large Language Models (LLMs) by providing robust error recovery for common LLM response quirks.

## What is WAIL?

WAIL (Widely Applicable Interface Language) is a schema language designed for:
1. Generating type-validated LLM prompts
2. Validating JSON responses from LLMs
3. Providing clear error messages for schema violations

## Features

- **Robust Error Recovery**: Handles common LLM response issues like trailing commas, unquoted identifiers, and malformed JSON
- **Type Validation**: Strong type checking for both schema definitions and JSON responses
- **High Performance**: Written in Rust with Python bindings for optimal speed
- **Developer Friendly**: Clear error messages and intuitive schema syntax
- **LLM-Optimized**: Specifically designed to work with the quirks and inconsistencies of LLM outputs

## Installation

```bash
pip install gasp-py
```

## Usage

```python
from gasp_py import WAILValidator

# Create a validator with your WAIL schema
validator = WAILValidator("""
    // Define your schema here
    template Response {
        name: String,
        age: Number,
        interests: Array<String>
    }
""")

# Validate JSON responses
try:
    validator.validate_json("""
    {
        "name": "Alice",
        "age": 25,
        "interests": ["coding", "AI", "music"]
    }
    """)
except Exception as e:
    print(f"Validation error: {e}")
```

## Error Recovery

GASP includes built-in error recovery for common LLM response issues:
- Trailing commas in arrays and objects
- Unquoted identifiers in object keys
- Missing quotes around strings
- Inconsistent whitespace and formatting

## License

Apache License, Version 2.0 - see LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
