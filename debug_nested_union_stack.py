#!/usr/bin/env python3
"""Debug nested union parsing with stack info."""

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
    item: Union[A, B]
    name: str
    
    def __repr__(self):
        return f"Container(name={getattr(self, 'name', None)}, item={getattr(self, 'item', None)})"


parser = Parser(Container)

xml_data = '''<Container>
    <item type="A">
        <name type="str">Nested A</name>
        <value_a type="int">100</value_a>
    </item>
    <name type="str">Container Name</name>
</Container>'''

# Feed all at once
result = parser.feed(xml_data)
print(f"\nFinal result: {result}")
if result:
    print(f"  Container.name: {getattr(result, 'name', None)}")
    print(f"  Container.item: {getattr(result, 'item', None)}")
    if hasattr(result, 'item') and result.item:
        print(f"    item.name: {getattr(result.item, 'name', None)}")
        print(f"    item.value_a: {getattr(result.item, 'value_a', None)}")
