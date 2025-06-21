#!/usr/bin/env python3
from typing import Union
from gasp import Parser, Deserializable

class A(Deserializable):
    name: str
    value_a: int

class B(Deserializable):
    title: str  
    value_b: float

# Named type alias
type NamedUnion = Union[A, B]

# Create parser and check tags
parser = Parser(NamedUnion)
print("Parser type_info:", parser.type_info)
print("Parser type_info.name:", parser.type_info.name if hasattr(parser, 'type_info') else 'N/A')
print("Parser type_info.kind:", parser.type_info.kind if hasattr(parser, 'type_info') else 'N/A')

# Check wanted tags
print("\nChecking wanted tags:")
try:
    wanted_tags = parser.get_wanted_tags()
    print(f"Wanted tags: {wanted_tags}")
except Exception as e:
    print(f"Error getting wanted tags: {e}")

# Test parsing
xml_data = '''<NamedUnion type="A">
    <name type="str">Named Test</name>
    <value_a type="int">100</value_a>
</NamedUnion>'''

print(f"\nParsing XML with tag 'NamedUnion'")
result = parser.feed(xml_data)
print(f"Feed result: {result}")
result = parser.validate()
print(f"Validate result: {result}")

# Also try with the union member tag
parser2 = Parser(NamedUnion)
xml_data2 = '''<A>
    <name type="str">Named Test</name>
    <value_a type="int">100</value_a>
</A>'''

print(f"\nParsing XML with tag 'A'")
result2 = parser2.feed(xml_data2)
print(f"Feed result: {result2}")
result2 = parser2.validate()
print(f"Validate result: {result2}")
