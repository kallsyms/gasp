#!/usr/bin/env python3
"""
Test script to verify that all components of the GASP package load correctly.
"""

import pytest
from typing import List, Optional
import gasp
from gasp.template_helpers import type_to_format_instructions, interpolate_prompt


class Person(gasp.Deserializable):
    """A person with name and age"""
    name: str
    age: int
    hobbies: Optional[List[str]] = None


def test_gasp_has_parser():
    """Test that Parser is available in gasp module"""
    assert hasattr(gasp, 'Parser'), "Parser not found"


def test_gasp_has_stream_parser():
    """Test that StreamParser is available in gasp module"""
    assert hasattr(gasp, 'StreamParser'), "StreamParser not found"


def test_gasp_has_deserializable():
    """Test that Deserializable is available in gasp module"""
    assert hasattr(gasp, 'Deserializable'), "Deserializable not found"


def test_gasp_has_template_helpers():
    """Test that template_helpers is available in gasp module"""
    assert hasattr(gasp, 'template_helpers'), "template_helpers not found"


def test_type_to_format_instructions():
    """Test type_to_format_instructions generates correct XML format"""
    instructions = type_to_format_instructions(Person)
    
    # Check that the instructions contain the expected XML tags
    assert "<Person>" in instructions
    assert "</Person>" in instructions
    assert "<name type=" in instructions
    assert "<age type=" in instructions
    assert "hobbies" in instructions


def test_interpolate_prompt():
    """Test interpolate_prompt correctly inserts format instructions"""
    template = "Create a person: {{return_type}}"
    prompt = interpolate_prompt(template, Person)
    
    # Check that the prompt contains the expected content
    assert "Your response should be formatted as:" in prompt
    assert "<Person>" in prompt
    assert "Create a person:" in prompt


def test_deserializable_from_partial():
    """Test creating Deserializable instance from partial data"""
    p = Person.__gasp_from_partial__({"name": "Alice", "age": 30})
    
    assert p.name == "Alice"
    assert p.age == 30
    assert p.hobbies is None  # Default value


def test_deserializable_from_partial_with_hobbies():
    """Test creating Deserializable instance with optional field"""
    p = Person.__gasp_from_partial__({
        "name": "Bob", 
        "age": 25,
        "hobbies": ["reading", "gaming"]
    })
    
    assert p.name == "Bob"
    assert p.age == 25
    assert p.hobbies == ["reading", "gaming"]


def test_deserializable_update():
    """Test updating Deserializable instance"""
    p = Person.__gasp_from_partial__({"name": "Alice", "age": 30})
    
    # Update the name
    p.__gasp_update__({"name": "Bob"})
    assert p.name == "Bob"
    assert p.age == 30  # Should remain unchanged
    
    # Update multiple fields
    p.__gasp_update__({"age": 35, "hobbies": ["cooking"]})
    assert p.name == "Bob"  # Should remain from previous update
    assert p.age == 35
    assert p.hobbies == ["cooking"]


def test_parser_with_person():
    """Test Parser with Person class"""
    parser = gasp.Parser(Person)
    
    xml_data = '''<Person>
    <name type="str">Charlie</name>
    <age type="int">40</age>
    <hobbies type="list">
        <item type="str">swimming</item>
        <item type="str">hiking</item>
    </hobbies>
</Person>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, Person)
    assert result.name == "Charlie"
    assert result.age == 40
    assert result.hobbies == ["swimming", "hiking"]


def test_parser_with_person_no_hobbies():
    """Test Parser with Person class without optional field"""
    parser = gasp.Parser(Person)
    
    xml_data = '''<Person>
    <name type="str">David</name>
    <age type="int">50</age>
</Person>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, Person)
    assert result.name == "David"
    assert result.age == 50
    assert result.hobbies is None


def test_stream_parser_basic():
    """Test StreamParser basic functionality"""
    stream_parser = gasp.StreamParser()
    
    # StreamParser is a low-level parser that doesn't take a type
    # It just parses JSON/XML data
    json_data = '{"name": "Eve", "age": 28}'
    
    result = stream_parser.parse(json_data)
    
    assert stream_parser.is_done()
    assert result is not None
    assert isinstance(result, dict)
    assert result["name"] == "Eve"
    assert result["age"] == 28


def test_model_dump():
    """Test model_dump functionality"""
    p = Person.__gasp_from_partial__({
        "name": "Frank",
        "age": 45,
        "hobbies": ["chess", "gardening"]
    })
    
    dumped = p.model_dump()
    
    assert isinstance(dumped, dict)
    assert dumped["name"] == "Frank"
    assert dumped["age"] == 45
    assert dumped["hobbies"] == ["chess", "gardening"]


def test_nested_deserializable():
    """Test nested Deserializable classes"""
    class Address(gasp.Deserializable):
        street: str
        city: str
        zip_code: str
    
    class Employee(gasp.Deserializable):
        name: str
        employee_id: int
        address: Address
    
    # Test creating nested structure
    emp = Employee.__gasp_from_partial__({
        "name": "Grace",
        "employee_id": 12345,
        "address": {
            "street": "123 Main St",
            "city": "Anytown",
            "zip_code": "12345"
        }
    })
    
    assert emp.name == "Grace"
    assert emp.employee_id == 12345
    assert isinstance(emp.address, Address)
    assert emp.address.street == "123 Main St"
    assert emp.address.city == "Anytown"
    assert emp.address.zip_code == "12345"
    
    # Test model_dump with nested structure
    dumped = emp.model_dump()
    assert dumped["address"]["street"] == "123 Main St"
    assert isinstance(dumped["address"], dict)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
