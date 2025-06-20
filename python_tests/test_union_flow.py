#!/usr/bin/env python3
"""Test to understand the flow of Union type handling."""

import pytest
from typing import Union, get_origin, get_args
from gasp import Parser, Deserializable
import json


class A(Deserializable):
    """First type in union"""
    name: str
    value_a: int
    
    def __repr__(self):
        return f"A(name={self.name}, value_a={self.value_a})"
    
    def __eq__(self, other):
        if not isinstance(other, A):
            return False
        return self.name == other.name and self.value_a == other.value_a


class B(Deserializable):
    """Second type in union"""
    title: str  
    value_b: float
    
    def __repr__(self):
        return f"B(title={self.title}, value_b={self.value_b})"
    
    def __eq__(self, other):
        if not isinstance(other, B):
            return False
        return self.title == other.title and self.value_b == other.value_b


# Named type alias
type NamedUnion = Union[A, B]


def test_named_union_with_type_attribute():
    """Test named Union with type attribute in XML"""
    parser = Parser(NamedUnion)
    
    xml_data = '''<NamedUnion type="A">
        <name type="str">Named Test</name>
        <value_a type="int">100</value_a>
    </NamedUnion>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, A)
    assert result.name == "Named Test"
    assert result.value_a == 100


def test_generic_union_with_type_attribute():
    """Test generic Union with type attribute in XML"""
    parser = Parser(Union[A, B])
    
    # For generic unions, the tag name should match the type being parsed
    xml_data = '''<A type="A">
        <name type="str">Generic Test</name>
        <value_a type="int">200</value_a>
    </A>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, A)
    assert result.name == "Generic Test"
    assert result.value_a == 200


def test_generic_union_type_b():
    """Test generic Union parsing type B"""
    parser = Parser(Union[A, B])
    
    xml_data = '''<B type="B">
        <title type="str">B Instance</title>
        <value_b type="float">3.14</value_b>
    </B>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, B)
    assert result.title == "B Instance"
    assert result.value_b == 3.14


def test_direct_class_parsing():
    """Test parsing directly to class A"""
    parser = Parser(A)
    
    xml_data = '''<A>
        <name type="str">Direct A</name>
        <value_a type="int">300</value_a>
    </A>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, A)
    assert result.name == "Direct A"
    assert result.value_a == 300


def test_manual_deserialization():
    """Test manual deserialization using __gasp_from_partial__"""
    data = {"name": "Manual Test", "value_a": 400}
    manual_a = A.__gasp_from_partial__(data)
    
    assert isinstance(manual_a, A)
    assert manual_a.name == "Manual Test"
    assert manual_a.value_a == 400


def test_union_with_tag_name_discrimination():
    """Test Union discrimination based on tag name"""
    parser = Parser(Union[A, B])
    
    # Tag name 'A' should select type A
    xml_a = '''<A>
        <name type="str">Tag-based A</name>
        <value_a type="int">500</value_a>
    </A>'''
    
    parser.feed(xml_a)
    result_a = parser.validate()
    
    assert isinstance(result_a, A)
    assert result_a.name == "Tag-based A"
    assert result_a.value_a == 500
    
    # Tag name 'B' should select type B
    parser_b = Parser(Union[A, B])
    xml_b = '''<B>
        <title type="str">Tag-based B</title>
        <value_b type="float">6.28</value_b>
    </B>'''
    
    parser_b.feed(xml_b)
    result_b = parser_b.validate()
    
    assert isinstance(result_b, B)
    assert result_b.title == "Tag-based B"
    assert result_b.value_b == 6.28


def test_union_in_list():
    """Test Union types within a list"""
    from typing import List
    
    parser = Parser(List[Union[A, B]])
    
    xml_data = '''<list type="list[Union[A, B]]">
        <item type="A">
            <name type="str">List A</name>
            <value_a type="int">600</value_a>
        </item>
        <item type="B">
            <title type="str">List B</title>
            <value_b type="float">7.5</value_b>
        </item>
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, list)
    assert len(result) == 2
    
    assert isinstance(result[0], A)
    assert result[0].name == "List A"
    assert result[0].value_a == 600
    
    assert isinstance(result[1], B)
    assert result[1].title == "List B"
    assert result[1].value_b == 7.5


def test_nested_union():
    """Test nested Union in a container class"""
    class Container(Deserializable):
        id: int
        content: Union[A, B]
    
    parser = Parser(Container)
    
    xml_data = '''<Container>
        <id type="int">1</id>
        <content type="A">
            <name type="str">Nested A</name>
            <value_a type="int">700</value_a>
        </content>
    </Container>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, Container)
    assert result.id == 1
    assert isinstance(result.content, A)
    assert result.content.name == "Nested A"
    assert result.content.value_a == 700


def test_union_type_introspection():
    """Test that we can introspect Union types"""
    union_type = Union[A, B]
    
    # Check origin and args
    assert get_origin(union_type) is Union
    args = get_args(union_type)
    assert len(args) == 2
    assert A in args
    assert B in args


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
