#!/usr/bin/env python3
"""Test field renaming for reserved keywords."""

from typing import Union
from gasp import Parser, Deserializable
import logging

# Enable debug logging
logging.basicConfig(level=logging.DEBUG, format='%(message)s')


class A(Deserializable):
    """First type in union"""
    name: str
    value_a: int
    
    def __repr__(self):
        return f"A(name={getattr(self, 'name', None)}, value_a={getattr(self, 'value_a', None)})"


class B(Deserializable):
    """Second type in union"""
    title: str  
    value_b: float


class Container(Deserializable):
    item: Union[A, B]  # This field name conflicts with reserved keyword "item"
    name: str
    
    def __repr__(self):
        return f"Container(name={getattr(self, 'name', None)}, item={getattr(self, 'item', None)})"


parser = Parser(Container)

# The XML should use the renamed field "_item" instead of "item"
xml_data = '''<Container>
    <_item type="A">
        <name type="str">Nested A</name>
        <value_a type="int">100</value_a>
    </_item>
    <name type="str">Container Name</name>
</Container>'''

print(f"Parsing XML with renamed field '_item':")
result = parser.feed(xml_data)
print(f"\nFinal result: {result}")
if result:
    print(f"  Container.name: {getattr(result, 'name', None)}")
    print(f"  Container.item: {getattr(result, 'item', None)}")
    if hasattr(result, 'item') and result.item:
        print(f"    item.name: {getattr(result.item, 'name', None)}")
        print(f"    item.value_a: {getattr(result.item, 'value_a', None)}")

# Also test with a Container that doesn't have conflicting field names
class Container2(Deserializable):
    data: Union[A, B]  # No conflict
    name: str
    
parser2 = Parser(Container2)

xml_data2 = '''<Container2>
    <data type="A">
        <name type="str">Nested A</name>
        <value_a type="int">200</value_a>
    </data>
    <name type="str">Container2 Name</name>
</Container2>'''

print(f"\n\nParsing XML without field name conflicts:")
result2 = parser2.feed(xml_data2)
print(f"\nFinal result: {result2}")
if result2:
    print(f"  Container2.name: {getattr(result2, 'name', None)}")
    print(f"  Container2.data: {getattr(result2, 'data', None)}")
