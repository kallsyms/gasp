#!/usr/bin/env python3
"""Debug nested union parsing."""

from typing import Union
from gasp import Parser, Deserializable


class A(Deserializable):
    """First type in union"""
    name: str
    value_a: int
    
    @classmethod
    def __gasp_from_partial__(cls, data: dict):
        print(f"A.__gasp_from_partial__ called with keys: {list(data.keys())}")
        instance = super().__gasp_from_partial__(data)
        print(f"  Created A: name={getattr(instance, 'name', None)}, value_a={getattr(instance, 'value_a', None)}")
        return instance


class B(Deserializable):
    """Second type in union"""
    title: str  
    value_b: float


class Container(Deserializable):
    item: Union[A, B]
    name: str
    
    @classmethod
    def __gasp_from_partial__(cls, data: dict):
        print(f"Container.__gasp_from_partial__ called with keys: {list(data.keys())}")
        instance = super().__gasp_from_partial__(data)
        print(f"  Created Container: name={getattr(instance, 'name', None)}, item={getattr(instance, 'item', None)}")
        return instance


parser = Parser(Container)

xml_data = '''<Container>
    <item type="A">
        <name type="str">Nested A</name>
        <value_a type="int">100</value_a>
    </item>
    <name type="str">Container Name</name>
</Container>'''

# Feed in chunks to see intermediate states
chunks = [
    '<Container>',
    '\n    <item type="A">',
    '\n        <name type="str">Nested A</name>',
    '\n        <value_a type="int">100</value_a>',
    '\n    </item>',
    '\n    <name type="str">Container Name</name>',
    '\n</Container>'
]

print("Feeding chunks:")
for i, chunk in enumerate(chunks):
    print(f"\nChunk {i}: {repr(chunk)}")
    result = parser.feed(chunk)
    if result:
        print(f"  Got result: {result}")
        if hasattr(result, 'name'):
            print(f"    name: {result.name}")
        if hasattr(result, 'item'):
            print(f"    item: {result.item}")

result = parser.validate()
print(f"\nFinal result: {result}")
if result:
    print(f"  name: {getattr(result, 'name', None)}")
    print(f"  item: {getattr(result, 'item', None)}")
    if hasattr(result, 'item') and result.item:
        print(f"    item.name: {getattr(result.item, 'name', None)}")
        print(f"    item.value_a: {getattr(result.item, 'value_a', None)}")
