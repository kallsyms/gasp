#!/usr/bin/env python3
from gasp import Parser, Deserializable
from typing import Union

class A(Deserializable):
    name: str
    value_a: int

class B(Deserializable):
    title: str
    value_b: float

class Container(Deserializable):
    item: Union[A, B]
    name: str

# Test XML data
xml_data = '''<Container>
    <item type="A">
        <name type="str">Nested A</name>
        <value_a type="int">100</value_a>
    </item>
    <name type="str">Container Name</name>
</Container>'''

# Enable debug logging
import logging
logging.basicConfig(level=logging.DEBUG)

# Create parser and feed data
parser = Parser(Container)
print(f"Initial parser state: {parser}")

# Feed data in chunks to see what happens
chunks = [
    '<Container>',
    '\n    <item type="A">',
    '\n        <name type="str">Nested A</name>',
    '\n        <value_a type="int">100</value_a>',
    '\n    </item>',
    '\n    <name type="str">Container Name</name>',
    '\n</Container>'
]

for i, chunk in enumerate(chunks):
    print(f"\n--- Feeding chunk {i}: {chunk!r} ---")
    result = parser.feed(chunk)
    print(f"Result after chunk: {result}")
    if result:
        print(f"  Result type: {type(result)}")
        if hasattr(result, '__dict__'):
            print(f"  Result dict: {result.__dict__}")

# Final validation
print("\n--- Final validation ---")
result = parser.validate()
print(f"Final result: {result}")
if result:
    print(f"Result type: {type(result)}")
    print(f"Result.item: {result.item}")
    print(f"Result.name: {result.name}")
    if hasattr(result, '__dict__'):
        print(f"Result dict: {result.__dict__}")
