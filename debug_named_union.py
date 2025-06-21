#!/usr/bin/env python3
from typing import Union, get_origin, get_args
from gasp import Parser, Deserializable
import sys

class A(Deserializable):
    name: str
    value_a: int

class B(Deserializable):
    title: str  
    value_b: float

# Named type alias
type NamedUnion = Union[A, B]

# Test type introspection
print("Type introspection:")
print(f"  NamedUnion: {NamedUnion}")
print(f"  get_origin(NamedUnion): {get_origin(NamedUnion)}")
print(f"  get_args(NamedUnion): {get_args(NamedUnion)}")
print(f"  NamedUnion.__name__: {getattr(NamedUnion, '__name__', 'NO __name__')}")
print(f"  repr(NamedUnion): {repr(NamedUnion)}")

# Create parser and check wanted tags
print("\nParser creation:")
parser = Parser(NamedUnion)
print("Parser created successfully")

# Test XML parsing
xml_data = '''<NamedUnion type="A">
    <name type="str">Named Test</name>
    <value_a type="int">100</value_a>
</NamedUnion>'''

print(f"\nParsing XML:\n{xml_data}")

result = parser.feed(xml_data)
print(f"\nFeed result: {result}")

result = parser.validate()
print(f"Validate result: {result}")

if result:
    print(f"Result type: {type(result)}")
    print(f"Result is A: {isinstance(result, A)}")
    if hasattr(result, 'name'):
        print(f"Result.name: {result.name}")
    if hasattr(result, 'value_a'):
        print(f"Result.value_a: {result.value_a}")
