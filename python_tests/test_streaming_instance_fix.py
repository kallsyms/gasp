#!/usr/bin/env python3
"""
Test streaming XML parsing with incremental field building
"""

import pytest
import gasp


class Fruit:
    name: str
    
    def __eq__(self, other):
        if not isinstance(other, Fruit):
            return False
        return self.name == other.name


class Product:
    name: str
    price: float
    category: str
    
    def __eq__(self, other):
        if not isinstance(other, Product):
            return False
        return (self.name == other.name and 
                self.price == other.price and 
                self.category == other.category)


class Item:
    value: str
    
    def __eq__(self, other):
        if not isinstance(other, Item):
            return False
        return self.value == other.value


def test_incremental_field_parsing():
    """Test incremental parsing of incomplete XML fields."""
    # Create a parser for Fruit type
    parser = gasp.Parser(Fruit)
    
    # Simulate streaming where field content is built up incrementally
    chunks = [
        '<Fruit>',
        '<name type="string">a',
        'pp',
        'le</n',
        'ame>',
        '</Fruit>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    # Verify we got a result
    assert result is not None
    assert hasattr(result, 'name')
    assert result.name == "apple"


def test_streaming_with_multiple_fields():
    """Test streaming with multiple fields being built incrementally."""
    parser = gasp.Parser(Product)
    
    # Simulate streaming that builds up multiple fields
    chunks = [
        '<Product>',
        '<name type="string">Lap',
        'top</name>',
        '<price type="float">999',
        '.99</price>',
        '<category type="string">Elec',
        'tronics</category>',
        '</Product>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    # Verify final result
    assert result is not None
    assert result.name == "Laptop"
    assert result.price == 999.99
    assert result.category == "Electronics"


def test_partial_tag_streaming():
    """Test streaming where even the XML tags themselves are split."""
    parser = gasp.Parser(Item)
    
    # Test where tags are split across chunks
    chunks = [
        '<It',
        'em>',
        '<val',
        'ue type="string">test</va',
        'lue>',
        '</Item>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    # Verify final result
    assert result is not None
    assert hasattr(result, 'value')
    assert result.value == "test"


def test_streaming_with_attributes_split():
    """Test streaming where attributes are split across chunks"""
    parser = gasp.Parser(Fruit)
    
    chunks = [
        '<Fruit>',
        '<name ty',
        'pe="str',
        'ing">ban',
        'ana</name>',
        '</Fruit>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert result.name == "banana"


def test_empty_field_streaming():
    """Test streaming with empty fields"""
    class EmptyTest:
        name: str
        description: str = ""
    
    parser = gasp.Parser(EmptyTest)
    
    chunks = [
        '<EmptyTest>',
        '<name type="string">Test</name>',
        '<description type="string"></description>',
        '</EmptyTest>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert result.name == "Test"
    assert result.description == ""


def test_nested_object_streaming():
    """Test streaming with nested objects"""
    class Inner:
        value: int
    
    class Outer:
        name: str
        inner: Inner
    
    parser = gasp.Parser(Outer)
    
    chunks = [
        '<Outer>',
        '<name type="string">Container</name>',
        '<inner type="Inner">',
        '<value type="int">42</value>',
        '</inner>',
        '</Outer>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert result.name == "Container"
    assert hasattr(result, 'inner')
    assert result.inner.value == 42


def test_list_streaming():
    """Test streaming with lists"""
    class ListContainer:
        items: list[str]
    
    parser = gasp.Parser(ListContainer)
    
    chunks = [
        '<ListContainer>',
        '<items type="list">',
        '<item type="string">first</item>',
        '<item type="string">sec',
        'ond</item>',
        '<item type="string">third</item>',
        '</items>',
        '</ListContainer>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert hasattr(result, 'items')
    assert result.items == ["first", "second", "third"]


def test_unicode_streaming():
    """Test streaming with unicode characters"""
    parser = gasp.Parser(Fruit)
    
    chunks = [
        '<Fruit>',
        '<name type="string">🍎 App',
        'le 🍏</name>',
        '</Fruit>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert result.name == "🍎 Apple 🍏"


def test_special_characters_streaming():
    """Test streaming with XML special characters"""
    parser = gasp.Parser(Item)
    
    chunks = [
        '<Item>',
        '<value type="string">Less &lt; Greater &gt; ',
        'Ampersand &amp; Quote &quot;</value>',
        '</Item>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert result.value == 'Less < Greater > Ampersand & Quote "'


def test_very_small_chunks():
    """Test streaming with very small chunks (character by character)"""
    parser = gasp.Parser(Fruit)
    
    xml = '<Fruit><name type="string">pear</name></Fruit>'
    
    result = None
    for char in xml:
        result = parser.feed(char)
    
    assert result is not None
    assert result.name == "pear"


def test_parser_completion_state():
    """Test parser completion state during streaming"""
    parser = gasp.Parser(Fruit)
    
    chunks = [
        '<Fruit>',
        '<name type="string">orange</name>',
        '</Fruit>'
    ]
    
    # Check completion state at each step
    assert not parser.is_complete()
    
    parser.feed(chunks[0])
    assert not parser.is_complete()
    
    parser.feed(chunks[1])
    assert not parser.is_complete()
    
    result = parser.feed(chunks[2])
    assert parser.is_complete()
    assert result is not None
    assert result.name == "orange"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
