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
logging.basicConfig(level=logging.DEBUG, format='%(levelname)s: %(message)s')

# Create parser and feed data
parser = Parser(Container)

# Feed in very small chunks to see detailed behavior
chunks = [
    '<Container>',
    '\n    <item type="A">',
    '\n        <name type="str">',
    'Nested A',
    '</name>',
    '\n        <value_a type="int">',
    '100',
    '</value_a>',
    '\n    </item>',
    '\n    <name type="str">',  # This is where Container's name field starts
    'Container Name',
    '</name>',
    '\n</Container>'
]

for i, chunk in enumerate(chunks):
    print(f"\n{'='*60}")
    print(f"Chunk {i}: {chunk!r}")
    print(f"{'='*60}")
    result = parser.feed(chunk)
    if result:
        print(f"Result: {result}")
        if hasattr(result, 'name'):
            print(f"  Container.name: {result.name}")
        if hasattr(result, 'item'):
            print(f"  Container.item: {result.item}")
            if hasattr(result.item, 'name'):
                print(f"    A.name: {result.item.name}")
            if hasattr(result.item, 'value_a'):
                print(f"    A.value_a: {result.item.value_a}")

# Final validation
print("\n" + "="*60)
print("FINAL VALIDATION")
print("="*60)
result = parser.validate()
if result:
    print(f"Result: {result}")
    print(f"  Container.name: {result.name}")
    print(f"  Container.item: {result.item}")
    print(f"    A.name: {result.item.name}")
    print(f"    A.value_a: {result.item.value_a}")
