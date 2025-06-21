#!/usr/bin/env python3
"""Debug named union parsing issue with detailed logging"""

import sys
import os
sys.path.insert(0, os.path.abspath('.'))

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


def test_with_raw_union():
    """Test with raw Union[A, B] for comparison"""
    print("=== Testing with raw Union[A, B] ===")
    parser = Parser(Union[A, B])
    
    # Try parsing with A tag
    xml_data = '''<A>
    <name type="str">Raw Union Test</name>
    <value_a type="int">42</value_a>
</A>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    print(f"Raw union result: {result}")
    print(f"Result type: {type(result)}")
    if result:
        print(f"Result attributes: {vars(result)}")
    print()


def test_named_union_as_member():
    """Test with NamedUnion but using member tag"""
    print("=== Testing NamedUnion with member tag ===")
    parser = Parser(NamedUnion)
    
    # Try parsing with A tag (union member)
    xml_data = '''<A>
    <name type="str">Member Tag Test</name>
    <value_a type="int">200</value_a>
</A>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    print(f"Member tag result: {result}")
    print(f"Result type: {type(result)}")
    if result:
        print(f"Result attributes: {vars(result)}")
    print()


def test_named_union_with_alias_tag():
    """Test with NamedUnion using the alias tag"""
    print("=== Testing NamedUnion with alias tag ===")
    parser = Parser(NamedUnion)
    
    xml_data = '''<NamedUnion type="A">
    <name type="str">Named Test</name>
    <value_a type="int">100</value_a>
</NamedUnion>'''
    
    print(f"Parsing XML:\n{xml_data}")
    
    parser.feed(xml_data)
    result = parser.validate()
    print(f"Alias tag result: {result}")
    print(f"Result type: {type(result)}")
    if result:
        print(f"Result attributes: {vars(result)}")
    print()


def test_type_detection():
    """Test type detection for NamedUnion"""
    print("=== Testing type detection ===")
    print(f"NamedUnion type: {type(NamedUnion)}")
    print(f"NamedUnion.__class__: {NamedUnion.__class__}")
    
    # Check if it has __value__ attribute (TypeAliasType)
    if hasattr(NamedUnion, '__value__'):
        print(f"NamedUnion.__value__: {NamedUnion.__value__}")
    
    # Check __name__ if it exists
    if hasattr(NamedUnion, '__name__'):
        print(f"NamedUnion.__name__: {NamedUnion.__name__}")
    
    # Check other attributes
    for attr in ['__origin__', '__args__', '__module__']:
        if hasattr(NamedUnion, attr):
            print(f"NamedUnion.{attr}: {getattr(NamedUnion, attr)}")
    print()


if __name__ == "__main__":
    test_type_detection()
    test_with_raw_union()
    test_named_union_as_member()
    test_named_union_with_alias_tag()
