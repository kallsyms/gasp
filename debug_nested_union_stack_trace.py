#!/usr/bin/env python3
from gasp import Parser, Deserializable
from typing import Union
import sys

class A(Deserializable):
    name: str
    value_a: int

class B(Deserializable):
    title: str
    value_b: float

class Container(Deserializable):
    item: Union[A, B]
    name: str

# Test XML data - Container with union field and then a name field
xml_data = '''<Container>
    <item type="A">
        <name type="str">Nested A</name>
        <value_a type="int">100</value_a>
    </item>
    <name type="str">Container Name</name>
</Container>'''

# Monkey patch the parser to add debug output
original_feed = Parser.feed

def debug_feed(self, chunk):
    print(f"\n=== FEED: {chunk!r} ===")
    # Call original with debug
    result = original_feed(self, chunk)
    # Print state after feed
    if hasattr(self, '_parser'):
        parser_obj = self._parser
        if hasattr(parser_obj, 'parser'):
            typed_parser = parser_obj.parser
            if hasattr(typed_parser, 'stack'):
                print(f"Stack depth: {len(typed_parser.stack)}")
                for i, frame in enumerate(typed_parser.stack):
                    print(f"  [{i}] {type(frame).__name__}")
                    if hasattr(frame, 'type_info'):
                        print(f"      type: {frame.type_info.name}")
                    if hasattr(frame, 'current_field'):
                        print(f"      current_field: {frame.current_field}")
                    if hasattr(frame, 'name'):
                        print(f"      name: {frame.name}")
    return result

Parser.feed = debug_feed

# Create parser and feed data
parser = Parser(Container)

# Feed in small chunks to see state changes
chunks = [
    '<Container>',
    '\n    <item type="A">',
    '\n        <name type="str">Nested A</name>',
    '\n        <value_a type="int">100</value_a>',
    '\n    </item>',
    '\n    <name type="str">Container Name</name>',
    '\n</Container>'
]

for chunk in chunks:
    result = parser.feed(chunk)
    if result:
        print(f"\nResult after chunk:")
        print(f"  Type: {type(result).__name__}")
        if hasattr(result, 'name'):
            print(f"  name: {result.name}")
        if hasattr(result, 'item'):
            print(f"  item: {result.item}")

# Final validation
print("\n=== FINAL VALIDATION ===")
result = parser.validate()
if result:
    print(f"Result: {result}")
    print(f"  Type: {type(result).__name__}")
    print(f"  name: {result.name}")
    print(f"  item: {result.item}")
    if hasattr(result.item, 'name'):
        print(f"  item.name: {result.item.name}")
    if hasattr(result.item, 'value_a'):
        print(f"  item.value_a: {result.item.value_a}")
