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

# Test with just the Container's name field
xml_data = '''<Container>
    <name type="str">Container Name</name>
</Container>'''

parser = Parser(Container)
result = parser.feed(xml_data)
print(f"Simple test - Container with just name field:")
print(f"  Result: {result}")
print(f"  Result.name: {result.name if result else 'N/A'}")

# Now test with item field first
xml_data2 = '''<Container>
    <item type="A">
        <name type="str">Nested A</name>
        <value_a type="int">100</value_a>
    </item>
    <name type="str">Container Name</name>
</Container>'''

parser2 = Parser(Container)
result2 = parser2.feed(xml_data2)
print(f"\nFull test - Container with item then name:")
print(f"  Result: {result2}")
print(f"  Result.item: {result2.item if result2 else 'N/A'}")
print(f"  Result.name: {result2.name if result2 else 'N/A'}")

# Test with name field first, then item
xml_data3 = '''<Container>
    <name type="str">Container Name</name>
    <item type="A">
        <name type="str">Nested A</name>
        <value_a type="int">100</value_a>
    </item>
</Container>'''

parser3 = Parser(Container)
result3 = parser3.feed(xml_data3)
print(f"\nReversed test - Container with name then item:")
print(f"  Result: {result3}")
print(f"  Result.name: {result3.name if result3 else 'N/A'}")
print(f"  Result.item: {result3.item if result3 else 'N/A'}")
