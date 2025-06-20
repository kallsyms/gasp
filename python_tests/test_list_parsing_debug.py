#!/usr/bin/env python3
"""Debug list parsing with detailed output"""

import pytest
from gasp import Parser, Deserializable
from typing import List
import json


class Person(Deserializable):
    """Person class for testing list parsing"""
    name: str
    age: int
    email: str = ""  # Optional with default
    
    def __repr__(self):
        return f"Person(name={self.name!r}, age={self.age}, email={self.email!r})"
    
    def __eq__(self, other):
        if not isinstance(other, Person):
            return False
        return self.name == other.name and self.age == other.age and self.email == other.email


def test_complete_list_parsing_xml():
    """Test parsing a complete list in XML format"""
    parser = Parser(List[Person])
    
    xml_data = '''<list type="list[Person]">
        <item type="Person">
            <name type="str">Alice</name>
            <age type="int">30</age>
            <email type="str">alice@example.com</email>
        </item>
        <item type="Person">
            <name type="str">Bob</name>
            <age type="int">25</age>
            <email type="str">bob@example.com</email>
        </item>
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, list)
    assert len(result) == 2
    assert parser.is_complete()
    
    # Check first person
    assert isinstance(result[0], Person)
    assert result[0].name == "Alice"
    assert result[0].age == 30
    assert result[0].email == "alice@example.com"
    
    # Check second person
    assert isinstance(result[1], Person)
    assert result[1].name == "Bob"
    assert result[1].age == 25
    assert result[1].email == "bob@example.com"


def test_list_with_default_values():
    """Test list parsing where some items use default values"""
    parser = Parser(List[Person])
    
    xml_data = '''<list type="list[Person]">
        <item type="Person">
            <name type="str">Carol</name>
            <age type="int">35</age>
        </item>
        <item type="Person">
            <name type="str">Dave</name>
            <age type="int">28</age>
            <email type="str">dave@example.com</email>
        </item>
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert len(result) == 2
    
    # Carol should have empty email (default)
    assert result[0].name == "Carol"
    assert result[0].age == 35
    assert result[0].email == ""
    
    # Dave has email specified
    assert result[1].name == "Dave"
    assert result[1].age == 28
    assert result[1].email == "dave@example.com"


def test_empty_list():
    """Test parsing an empty list"""
    parser = Parser(List[Person])
    
    xml_data = '''<list type="list[Person]">
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, list)
    assert len(result) == 0
    assert parser.is_complete()


def test_list_of_primitives():
    """Test parsing lists of primitive types"""
    # Test list of strings
    parser_str = Parser(List[str])
    xml_str = '''<list type="list[str]">
        <item type="str">apple</item>
        <item type="str">banana</item>
        <item type="str">cherry</item>
    </list>'''
    
    parser_str.feed(xml_str)
    result_str = parser_str.validate()
    
    assert result_str == ["apple", "banana", "cherry"]
    
    # Test list of integers
    parser_int = Parser(List[int])
    xml_int = '''<list type="list[int]">
        <item type="int">1</item>
        <item type="int">2</item>
        <item type="int">3</item>
    </list>'''
    
    parser_int.feed(xml_int)
    result_int = parser_int.validate()
    
    assert result_int == [1, 2, 3]


def test_nested_lists():
    """Test parsing nested lists"""
    parser = Parser(List[List[int]])
    
    xml_data = '''<list type="list[list[int]]">
        <item type="list[int]">
            <item type="int">1</item>
            <item type="int">2</item>
        </item>
        <item type="list[int]">
            <item type="int">3</item>
            <item type="int">4</item>
            <item type="int">5</item>
        </item>
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert len(result) == 2
    assert result[0] == [1, 2]
    assert result[1] == [3, 4, 5]


def test_list_streaming():
    """Test streaming list parsing"""
    parser = Parser(List[Person])
    
    chunks = [
        '<list type="list[Person]">',
        '<item type="Person">',
        '<name type="str">Eve</name>',
        '<age type="int">40</age>',
        '</item>',
        '<item type="Person">',
        '<name type="str">Frank</name>',
        '<age type="int">45</age>',
        '</item>',
        '</list>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert parser.is_complete()
    validated = parser.validate()
    
    assert validated is not None
    assert len(validated) == 2
    assert validated[0].name == "Eve"
    assert validated[0].age == 40
    assert validated[1].name == "Frank"
    assert validated[1].age == 45


def test_list_with_mixed_content():
    """Test list with complex nested objects"""
    class Address(Deserializable):
        street: str
        city: str
        zip_code: str
    
    class Employee(Deserializable):
        name: str
        addresses: List[Address]
    
    parser = Parser(List[Employee])
    
    xml_data = '''<list type="list[Employee]">
        <item type="Employee">
            <name type="str">John</name>
            <addresses type="list[Address]">
                <item type="Address">
                    <street type="str">123 Main St</street>
                    <city type="str">Anytown</city>
                    <zip_code type="str">12345</zip_code>
                </item>
                <item type="Address">
                    <street type="str">456 Oak Ave</street>
                    <city type="str">Other City</city>
                    <zip_code type="str">67890</zip_code>
                </item>
            </addresses>
        </item>
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert len(result) == 1
    assert result[0].name == "John"
    assert len(result[0].addresses) == 2
    assert result[0].addresses[0].street == "123 Main St"
    assert result[0].addresses[1].city == "Other City"


def test_list_single_item():
    """Test list with a single item"""
    parser = Parser(List[Person])
    
    xml_data = '''<list type="list[Person]">
        <item type="Person">
            <name type="str">Solo</name>
            <age type="int">100</age>
        </item>
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert len(result) == 1
    assert result[0].name == "Solo"
    assert result[0].age == 100


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
