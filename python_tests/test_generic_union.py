#!/usr/bin/env python3
"""Test case for generic Union parsing and deserialization."""

import pytest
from typing import Union, List
from gasp import Parser, Deserializable


class A(Deserializable):
    """First type in union"""
    name: str
    value_a: int
    
    def __repr__(self):
        return f"A(name='{self.name}', value_a={self.value_a})"
    
    def __eq__(self, other):
        if not isinstance(other, A):
            return False
        return self.name == other.name and self.value_a == other.value_a


class B(Deserializable):
    """Second type in union"""
    title: str  
    value_b: float
    
    def __repr__(self):
        return f"B(title='{self.title}', value_b={self.value_b})"
    
    def __eq__(self, other):
        if not isinstance(other, B):
            return False
        return self.title == other.title and self.value_b == other.value_b


def test_generic_union():
    """Test generic Union with type attribute"""
    # Create parser with generic Union type
    parser = Parser(Union[A, B])
    
    # Test case A - XML format (type attribute is redundant but allowed)
    response_a = '''<A>
        <name type="str">Test A</name>
        <value_a type="int">42</value_a>
    </A>'''
    parser.feed(response_a)
    result_a = parser.validate()
    
    assert result_a is not None
    assert isinstance(result_a, A)
    assert result_a.name == "Test A"
    assert result_a.value_a == 42
    
    # Test case B - XML format
    parser_b = Parser(Union[A, B])
    response_b = '''<B>
        <title type="str">Test B</title>
        <value_b type="float">3.14</value_b>
    </B>'''
    parser_b.feed(response_b)
    result_b = parser_b.validate()
    
    assert result_b is not None
    assert isinstance(result_b, B)
    assert result_b.title == "Test B"
    assert result_b.value_b == 3.14


def test_generic_union_no_typename():
    """Test generic Union without _type_name, using tag name for discrimination"""
    # Create parser with generic Union type
    parser = Parser(Union[A, B])
    
    # Test case A (based on field matching) - tag name determines type
    response_a = '''<A>
        <name type="str">Test A</name>
        <value_a type="int">42</value_a>
    </A>'''
    parser.feed(response_a)
    result_a = parser.validate()
    
    assert result_a is not None
    assert isinstance(result_a, A)
    assert result_a.name == "Test A"
    assert result_a.value_a == 42
    
    # Test case B (based on field matching) - tag name determines type
    parser_b = Parser(Union[A, B])
    response_b = '''<B>
        <title type="str">Test B</title>
        <value_b type="float">3.14</value_b>
    </B>'''
    parser_b.feed(response_b)
    result_b = parser_b.validate()
    
    assert result_b is not None
    assert isinstance(result_b, B)
    assert result_b.title == "Test B"
    assert result_b.value_b == 3.14


# Named type alias (for comparison)
type MyUnion = Union[A, B]


def test_named_union():
    """Test named Union type alias"""
    parser = Parser(MyUnion)
    
    # Test case A - correct format uses class name as tag
    response_a = '''<A>
        <name type="str">Test A</name>
        <value_a type="int">42</value_a>
    </A>'''
    parser.feed(response_a)
    result_a = parser.validate()
    
    assert result_a is not None
    assert isinstance(result_a, A)
    assert result_a.name == "Test A"
    assert result_a.value_a == 42


def test_list_of_mixed_union_items():
    """Test List of mixed Union items"""
    parser = Parser(List[Union[A, B]])

    mixed_list_data = '''<list type="list[Union[A, B]]">
        <item type="A">
            <name type="str">First A</name>
            <value_a type="int">123</value_a>
        </item>
        <item type="B">
            <title type="str">First B</title>
            <value_b type="float">45.67</value_b>
        </item>
        <item type="A">
            <name type="str">Second A</name>
            <value_a type="int">890</value_a>
        </item>
    </list>'''
    parser.feed(mixed_list_data)
    result = parser.validate()

    assert result is not None
    assert isinstance(result, list)
    assert len(result) == 3

    # First item (A)
    assert isinstance(result[0], A)
    assert result[0].name == "First A"
    assert result[0].value_a == 123

    # Second item (B)
    assert isinstance(result[1], B)
    assert result[1].title == "First B"
    assert result[1].value_b == 45.67

    # Third item (A)
    assert isinstance(result[2], A)
    assert result[2].name == "Second A"
    assert result[2].value_a == 890


def test_union_streaming():
    """Test streaming parsing of Union types"""
    parser = Parser(Union[A, B])
    
    chunks = [
        '<A>',
        '<name type="str">Streaming A</name>',
        '<value_a type="int">999</value_a>',
        '</A>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    validated = parser.validate()
    assert validated is not None
    assert isinstance(validated, A)
    assert validated.name == "Streaming A"
    assert validated.value_a == 999


def test_union_with_optional_fields():
    """Test Union types with optional fields"""
    class C(Deserializable):
        required: str
        optional: int = 10
    
    class D(Deserializable):
        name: str
        count: int = 0
    
    parser = Parser(Union[C, D])
    
    # Test C with optional field present
    xml_c = '''<C>
        <required type="str">Required value</required>
        <optional type="int">20</optional>
    </C>'''
    parser.feed(xml_c)
    result_c = parser.validate()
    
    assert isinstance(result_c, C)
    assert result_c.required == "Required value"
    assert result_c.optional == 20
    
    # Test D with optional field missing
    parser_d = Parser(Union[C, D])
    xml_d = '''<D>
        <name type="str">Test D</name>
    </D>'''
    parser_d.feed(xml_d)
    result_d = parser_d.validate()
    
    assert isinstance(result_d, D)
    assert result_d.name == "Test D"
    assert result_d.count == 0  # Default value


def test_union_error_handling():
    """Test error handling for Union types"""
    parser = Parser(Union[A, B])
    
    # Test with invalid type
    invalid_xml = '''<C>
        <unknown type="str">Invalid</unknown>
    </C>'''
    parser.feed(invalid_xml)
    result = parser.validate()
    
    # Should return None or handle gracefully
    # The exact behavior depends on the implementation
    # but it shouldn't crash


def test_nested_unions():
    """Test nested Union types"""
    class Container(Deserializable):
        item: Union[A, B]
        name: str
    
    parser = Parser(Container)
    
    xml_data = '''<Container>
        <item type="A">
            <name type="str">Nested A</name>
            <value_a type="int">100</value_a>
        </item>
        <name type="str">Container Name</name>
    </Container>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, Container)
    assert result.name == "Container Name"
    assert isinstance(result.item, A)
    assert result.item.name == "Nested A"
    assert result.item.value_a == 100


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
