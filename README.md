# GASP - A Type-Safe, Streaming XML-to-Object Parser for Python

GASP is a library for parsing structured XML from Large Language Model (LLM) outputs directly into typed Python objects. It uses your Python type hints to drive the parsing process, providing a robust and intuitive way to work with structured data from LLMs.

## The Problem

LLMs are proficient at generating structured data, but they often embed it within natural language and the format can be inconsistent. You want to extract a complex Python object, but you get something like this:

```
Of course! Here is the information about the person you requested:

<Person>
  <name>Alice</name>
  <age>30</age>
  <hobbies>
    <item>coding</item>
    <item>hiking</item>
  </hobbies>
</Person>

I hope this is helpful!
```

GASP is designed to solve this exact problem. It ignores the surrounding text, finds the `<Person>` tag, and uses your `Person` class definition to parse the nested XML into a valid Python object.

## How GASP Works

GASP uses a type-directed XML parsing strategy. The structure of your Python classes tells the parser what to expect in the XML stream.

1.  **Type-Driven Matching**: GASP maps your Python class names to XML root tags (e.g., `class Person` matches `<Person>`).
2.  **Field-Driven Parsing**: It maps class attribute names to nested XML tags (e.g., `person.name` matches `<name>...</name>`).
3.  **Streaming & Incremental**: The parser processes data as it arrives, handling XML tags that are split across multiple chunks.
4.  **Object Instantiation**: It incrementally builds your Python objects as it successfully parses the XML, setting attributes as their corresponding tags are closed.

## Features

-   **Type-Directed Parsing**: Your Python type hints (`str`, `int`, `List`, `Union`, custom classes) are the single source of truth for the parser.
-   **Streaming XML Engine**: A high-performance Rust core handles partial data and tags split across chunks, making it ideal for real-time applications.
-   **Nested Object & Collection Support**: Naturally parses complex, nested XML structures into corresponding Python objects and collections (`List`, `Dict`, `Set`, `Tuple`).
-   **Union Type Resolution**: Intelligently selects the correct type from a `Union` based on the XML tag encountered in the stream.
-   **Automatic Tag Filtering**: Ignores common LLM "thinking" tags (e.g., `<think>`, `<system>`) by default, so you only get the data you want.
-   **Pydantic Integration**: First-class support for parsing data directly into Pydantic models.

## Installation

```bash
pip install gasp-py
```

## Quick Example

Define your Python class. The class name and attribute names will be used to match the XML tags.

```python
# models.py
from typing import List

class Person:
    name: str
    age: int
    hobbies: List[str]
```

Now, use the `Parser` to process LLM output containing XML that matches your class structure.

```python
from gasp import Parser
from models import Person

llm_output = """
Here is the data you requested.
<Person>
  <name>Alice</name>
  <age>30</age>
  <hobbies>
    <item>coding</item>
    <item>hiking</item>
  </hobbies>
</Person>
"""

# Create a parser for the Person type
parser = Parser(Person)

# Feed the LLM output to the parser
parser.feed(llm_output)
person = parser.validate()

# The 'person' variable is now a fully typed Python object
print(f"{person.name} is {person.age} and enjoys {', '.join(person.hobbies)}.")
# Output: Alice is 30 and enjoys coding, hiking.
```

## XML Structure Guide

GASP expects the XML structure to mirror your Python class definitions.

### Primitives

Primitive types are parsed from the text content of a tag.

```python
class Book:
    title: str  # <title>The Great Gatsby</title>
    pages: int  # <pages>180</pages>
```

### Lists and Sets

Lists and Sets are parsed from a container tag containing multiple `<item>` tags.

```python
class ShoppingList:
    items: List[str]

# Corresponds to:
# <items>
#   <item>milk</item>
#   <item>bread</item>
# </items>
```

### Dictionaries

Dictionaries are parsed from a container tag with `<item>` tags that have a `key` attribute.

```python
class Config:
    settings: Dict[str, str]

# Corresponds to:
# <settings>
#   <item key="theme">dark</item>
#   <item key="font_size">14</item>
# </settings>
```

### Nested Objects

Nested objects are handled by nesting XML tags that match the class and attribute names.

```python
class Employee:
    name: str
    role: str

class Company:
    name: str
    employees: List[Employee]

# Corresponds to:
# <Company>
#   <name>TechCorp</name>
#   <employees>
#     <item>
#       <Employee>
#         <name>Alice</name>
#         <role>Engineer</role>
#       </Employee>
#     </item>
#     <item>
#       <Employee>
#         <name>Bob</name>
#         <role>Designer</role>
#       </Employee>
#     </item>
#   </employees>
# </Company>
```

## Advanced Usage

### Union Types

GASP can distinguish between types in a `Union` based on the XML tag.

```python
class Success:
    data: str

class Error:
    message: str

ResponseType = Union[Success, Error]

# The parser will instantiate a `Success` object
parser = Parser(ResponseType)
parser.feed("<Success><data>Operation complete.</data></Success>")

# The parser will instantiate an `Error` object
parser = Parser(ResponseType)
parser.feed("<Error><message>Permission denied.</message></Error>")
```

### Template Generation

You can generate XML format instructions to include in your prompts, guiding the LLM to produce the correct output.

```python
from gasp.template_helpers import interpolate_prompt

template = "Please generate the data in the following format:\n{{return_type}}"
prompt = interpolate_prompt(template, Company)
print(prompt)
```

This will generate a prompt with a clear XML schema for the LLM to follow.

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.

## License

Apache License, Version 2.0
