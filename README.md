# GASP - Type-Safe LLM Output Parser

GASP is a Rust-based parser for turning LLM outputs into properly typed Python objects. It handles streaming JSON fragments, recovers from common LLM quirks, and makes structured data extraction actually pleasant.

## The Problem

LLMs are great at generating structured data when asked, but not perfect:

```
<Person>
{
  "name": "Alice Smith",
  "age": 30,
  hobbies: ["coding", "hiking"]
}
</Person>
```

That output has unquoted keys, inconsistent formatting, and is embedded in natural language. Most JSON parsers just give up.

## How GASP Works

GASP uses a tag-based approach to extract and type-cast structured data:

1. Tags like `<Person>...</Person>` mark where the structured data lives (and what type it is)
2. The parser ignores everything outside those tags
3. Inside the tags, it handles messy JSON with broken quotes, trailing commas, etc.
4. The data gets converted into proper Python objects based on type annotations

## Features

- **Tag-Based Extraction**: Extract structured data even when surrounded by explanatory text
- **Streaming Support**: Process data incrementally as it arrives from the LLM
- **Type Inference**: Automatically match JSON objects to Python classes
- **Error Recovery**: Handle common JSON mistakes that LLMs make
- **Pydantic Integration**: Works with Pydantic for validation and schema definition

## Installation

```bash
pip install gasp-py
```

## Quick Example

```python
from gasp import Parser, Deserializable
from typing import List, Optional

class Address(Deserializable):
    street: str
    city: str
    zip_code: str

class Person(Deserializable):
    name: str
    age: int
    address: Address
    hobbies: Optional[List[str]] = None

# Create a parser for the Person type
parser = Parser(Person)

# Process LLM output chunks as they arrive
chunks = [
    '<Person>{"name": "Alice", "age": 30',
    ', "address": {"street": "123 Main St", "city": "Springfield"',
    ', "zip_code": "12345"}, "hobbies": ["reading", "coding"]}</Person>'
]

for chunk in chunks:
    result = parser.feed(chunk)
    print(result)  # Will show partial objects as they're built

# Get the final validated result
person = parser.validate()
print(f"Hello {person.name}!")  # Hello Alice!
```

## Working with Pydantic

GASP integrates seamlessly with Pydantic:

```python
from pydantic import BaseModel
from gasp import Parser

class UserProfile(BaseModel):
    username: str
    email: str
    is_active: bool = True

# Create parser from Pydantic model
parser = Parser.from_pydantic(UserProfile)

# Feed LLM output with tags
llm_output = '<UserProfile>{"username": "alice42", "email": "alice@example.com"}</UserProfile>'
result = parser.feed(llm_output)

# Access as a proper Pydantic object
profile = UserProfile.model_validate(parser.validate())
print(profile.model_dump_json(indent=2))
```

## How Tags Work

The tag name directly indicates what Python type to instantiate:

```
<Person>{ ... JSON data ... }</Person>  # Creates a Person instance
<List>[ ... array data ... ]</List>     # Creates a List
<Address>{ ... address data ... }</Address>  # Creates an Address
```

The parser ignores everything outside of the tags, so the LLM can provide explanations, context, or other text alongside the structured data.

## Customizing Behavior

Need more control? You can customize type conversion, validation, and parsing behavior:

```python
# Custom type conversions and validation
class CustomPerson(Deserializable):
    name: str
    age: int

    @classmethod
    def __gasp_from_partial__(cls, partial_data):
        """Add custom validation or pre-processing"""
        # Normalize name to title case
        if "name" in partial_data:
            partial_data["name"] = partial_data["name"].title()
        return super().__gasp_from_partial__(partial_data)
```

## Contributing

Contributions welcome! Check out the examples directory to see how things work.

## License

Apache License, Version 2.0
