#!/usr/bin/env python3
"""
Test that container types (tuple, dict, set) work correctly with XML format
"""

import pytest
from gasp import Parser
from typing import Tuple, Dict, Set


def test_basic_tuple():
    """Test basic tuple parsing"""
    parser = Parser(tuple)
    xml = '''<tuple type="tuple">
    <item type="int">1</item>
    <item type="int">2</item>
    <item type="int">3</item>
</tuple>'''
    result = parser.feed(xml)
    
    assert result == (1, 2, 3)
    assert isinstance(result, tuple)
    assert type(result) == tuple


def test_typed_tuples():
    """Test typed tuple parsing"""
    # Test homogeneous tuple (Tuple[int, ...])
    parser = Parser(Tuple[int, ...])
    xml = '''<tuple type="tuple[int, ...]">
    <item type="int">1</item>
    <item type="int">2</item>
    <item type="int">3</item>
    <item type="int">4</item>
    <item type="int">5</item>
</tuple>'''
    result = parser.feed(xml)
    
    assert result == (1, 2, 3, 4, 5)
    assert isinstance(result, tuple)
    
    # Test fixed tuple (Tuple[str, int, float])
    parser = Parser(Tuple[str, int, float])
    xml = '''<tuple type="tuple[str, int, float]">
    <item type="str">hello</item>
    <item type="int">42</item>
    <item type="float">3.14</item>
</tuple>'''
    result = parser.feed(xml)
    
    assert result == ("hello", 42, 3.14)
    assert isinstance(result, tuple)


def test_nested_tuples():
    """Test tuples containing complex types"""
    # Tuple of lists
    parser = Parser(Tuple[list, list])
    xml = '''<tuple type="tuple[list, list]">
    <item type="list">
        <item type="int">1</item>
        <item type="int">2</item>
        <item type="int">3</item>
    </item>
    <item type="list">
        <item type="str">a</item>
        <item type="str">b</item>
        <item type="str">c</item>
    </item>
</tuple>'''
    result = parser.feed(xml)
    
    assert result == ([1, 2, 3], ["a", "b", "c"])
    assert isinstance(result, tuple)
    assert isinstance(result[0], list)
    assert isinstance(result[1], list)


class Person:
    """Simple person class for testing"""
    def __init__(self, name: str = "", age: int = 0):
        self.name = name
        self.age = age
    
    def __repr__(self):
        return f"Person(name='{self.name}', age={self.age})"
    
    def __eq__(self, other):
        if not isinstance(other, Person):
            return False
        return self.name == other.name and self.age == other.age


def test_tuple_with_objects():
    """Test tuples containing objects"""
    # Tuple of Person objects
    parser = Parser(Tuple[Person, Person])
    xml = '''<tuple type="tuple[Person, Person]">
    <item type="Person">
        <name type="str">Alice</name>
        <age type="int">30</age>
    </item>
    <item type="Person">
        <name type="str">Bob</name>
        <age type="int">25</age>
    </item>
</tuple>'''
    result = parser.feed(xml)
    
    assert isinstance(result, tuple)
    assert len(result) == 2
    assert isinstance(result[0], Person)
    assert isinstance(result[1], Person)
    assert result[0].name == "Alice"
    assert result[0].age == 30
    assert result[1].name == "Bob"
    assert result[1].age == 25


def test_streaming_tuple():
    """Test streaming tuple parsing"""
    parser = Parser(Tuple[str, int, bool])
    
    chunks = [
        '<tuple type="tuple[str, int, bool]">',
        '<item type="str">streaming</item>',
        '<item type="int">123</item>',
        '<item type="bool">true</item>',
        '</tuple>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result == ("streaming", 123, True)
    assert parser.is_complete()


def test_tuple_vs_list():
    """Compare tuple and list parsing"""
    # Same data, different types
    list_xml = '''<list type="list">
    <item type="int">1</item>
    <item type="int">2</item>
    <item type="int">3</item>
</list>'''
    
    tuple_xml = '''<tuple type="tuple">
    <item type="int">1</item>
    <item type="int">2</item>
    <item type="int">3</item>
</tuple>'''
    
    # Parse as list
    list_parser = Parser(list)
    list_result = list_parser.feed(list_xml)
    
    assert list_result == [1, 2, 3]
    assert isinstance(list_result, list)
    
    # Parse as tuple
    tuple_parser = Parser(tuple)
    tuple_result = tuple_parser.feed(tuple_xml)
    
    assert tuple_result == (1, 2, 3)
    assert isinstance(tuple_result, tuple)


def test_dict_support():
    """Test dict parsing"""
    # Test Dict[str, int]
    parser = Parser(Dict[str, int])
    xml = '''<dict type="dict[str, int]">
    <item key="a" type="int">1</item>
    <item key="b" type="int">2</item>
    <item key="c" type="int">3</item>
</dict>'''
    result = parser.feed(xml)
    
    assert result == {"a": 1, "b": 2, "c": 3}
    assert isinstance(result, dict)
    assert list(result.keys()) == ["a", "b", "c"]
    assert list(result.values()) == [1, 2, 3]
    
    # Test plain dict
    parser2 = Parser(dict)
    result2 = parser2.feed(xml)
    
    assert result2 == {"a": 1, "b": 2, "c": 3}
    assert isinstance(result2, dict)


def test_set_support():
    """Test set parsing"""
    # Test Set[int]
    parser = Parser(Set[int])
    xml = '''<set type="set[int]">
    <item type="int">1</item>
    <item type="int">2</item>
    <item type="int">3</item>
    <item type="int">2</item>
</set>'''
    result = parser.feed(xml)
    
    assert result == {1, 2, 3}
    assert isinstance(result, set)
    assert len(result) == 3  # Should be 3 due to duplicate
    
    # Test plain set
    parser2 = Parser(set)
    result2 = parser2.feed(xml)
    
    assert result2 == {1, 2, 3}
    assert isinstance(result2, set)


def test_dict_with_different_formats():
    """Test dict parsing with different XML formats"""
    parser = Parser(Dict[str, int])
    
    # Test simple format
    xml1 = '<dict type="dict"><item key="a" type="int">1</item></dict>'
    result1 = parser.feed(xml1)
    assert result1 == {"a": 1}
    
    # Test without explicit type on dict tag
    parser2 = Parser(dict)
    xml2 = '<dict><item key="a" type="int">1</item></dict>'
    result2 = parser2.feed(xml2)
    assert result2 == {"a": 1}


def test_set_with_different_formats():
    """Test set parsing with different XML formats"""
    parser = Parser(Set[int])
    
    # Test simple format
    xml1 = '<set type="set"><item type="int">1</item></set>'
    result1 = parser.feed(xml1)
    assert result1 == {1}
    
    # Test without explicit type on set tag
    parser2 = Parser(set)
    xml2 = '<set><item type="int">1</item></set>'
    result2 = parser2.feed(xml2)
    assert result2 == {1}


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
