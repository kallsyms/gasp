#!/usr/bin/env python3
"""
Test for scalar type parsing in GASP
Tests all basic scalar types: str, int, float, bool
"""

import pytest
from gasp import Parser


def test_string_parsing():
    """Test string parsing"""
    # Test with XML tags
    parser = Parser(str)
    xml_data = '''<value type="string">Hello, world!</value>'''
    
    result = parser.feed(xml_data)
    assert result == "Hello, world!"
    assert isinstance(result, str)
    assert parser.is_complete()
    
    # Test with str tag
    parser2 = Parser(str)
    xml_data2 = '''<str type="string">Simple string</str>'''
    
    result2 = parser2.feed(xml_data2)
    assert result2 == "Simple string"
    assert isinstance(result2, str)
    assert parser2.is_complete()


def test_int_parsing():
    """Test integer parsing"""
    # Test with XML tags
    parser = Parser(int)
    xml_data = '''<value type="int">42</value>'''
    
    result = parser.feed(xml_data)
    assert result == 42
    assert isinstance(result, int)
    assert parser.is_complete()
    
    # Test with int tag
    parser2 = Parser(int)
    xml_data2 = '''<int type="int">123</int>'''
    
    result2 = parser2.feed(xml_data2)
    assert result2 == 123
    assert isinstance(result2, int)
    assert parser2.is_complete()
    
    # Test negative integer
    parser3 = Parser(int)
    xml_data3 = '''<value type="int">-456</value>'''
    
    result3 = parser3.feed(xml_data3)
    assert result3 == -456
    assert isinstance(result3, int)


def test_float_parsing():
    """Test float parsing"""
    # Test with XML tags
    parser = Parser(float)
    xml_data = '''<value type="float">3.14159</value>'''
    
    result = parser.feed(xml_data)
    assert result is not None
    assert abs(result - 3.14159) < 0.0001
    assert isinstance(result, float)
    assert parser.is_complete()
    
    # Test with float tag
    parser2 = Parser(float)
    xml_data2 = '''<float type="float">2.71828</float>'''
    
    result2 = parser2.feed(xml_data2)
    assert result2 is not None
    assert abs(result2 - 2.71828) < 0.0001
    assert isinstance(result2, float)
    assert parser2.is_complete()
    
    # Test negative float
    parser3 = Parser(float)
    xml_data3 = '''<value type="float">-1.5</value>'''
    
    result3 = parser3.feed(xml_data3)
    assert result3 == -1.5
    assert isinstance(result3, float)


def test_bool_parsing():
    """Test boolean parsing"""
    # Test True with XML tags
    parser = Parser(bool)
    xml_data = '''<value type="bool">true</value>'''
    
    result = parser.feed(xml_data)
    assert result is True
    assert isinstance(result, bool)
    assert parser.is_complete()
    
    # Test False with bool tag
    parser2 = Parser(bool)
    xml_data2 = '''<bool type="bool">false</bool>'''
    
    result2 = parser2.feed(xml_data2)
    assert result2 is False
    assert isinstance(result2, bool)
    assert parser2.is_complete()
    
    # Test with different case
    parser3 = Parser(bool)
    xml_data3 = '''<value type="bool">True</value>'''
    
    result3 = parser3.feed(xml_data3)
    assert result3 is True
    assert isinstance(result3, bool)
    
    # Test False with different case
    parser4 = Parser(bool)
    xml_data4 = '''<value type="bool">False</value>'''
    
    result4 = parser4.feed(xml_data4)
    assert result4 is False
    assert isinstance(result4, bool)


def test_none_parsing():
    """Test null/None parsing"""
    # Test with None type
    parser = Parser(type(None))
    xml_data = '''<value type="null">null</value>'''
    
    result = parser.feed(xml_data)
    assert result is None
    assert parser.is_complete()
    
    # Test with empty value
    parser2 = Parser(type(None))
    xml_data2 = '''<value type="null"></value>'''
    
    result2 = parser2.feed(xml_data2)
    assert result2 is None


def test_streaming_scalar_parsing():
    """Test streaming parsing of scalar types"""
    # Test streaming string
    parser = Parser(str)
    chunks = [
        '<value type="string">',
        'Hello, ',
        'streaming ',
        'world!',
        '</value>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result == "Hello, streaming world!"
    assert parser.is_complete()
    
    # Test streaming integer
    parser2 = Parser(int)
    chunks2 = [
        '<value type="int">',
        '12345',
        '</value>'
    ]
    
    result2 = None
    for chunk in chunks2:
        result2 = parser2.feed(chunk)
    
    assert result2 == 12345
    assert parser2.is_complete()


def test_edge_cases():
    """Test edge cases for scalar parsing"""
    # Test empty string
    parser = Parser(str)
    xml_data = '''<value type="string"></value>'''
    
    result = parser.feed(xml_data)
    assert result == ""
    assert isinstance(result, str)
    
    # Test zero integer
    parser2 = Parser(int)
    xml_data2 = '''<value type="int">0</value>'''
    
    result2 = parser2.feed(xml_data2)
    assert result2 == 0
    assert isinstance(result2, int)
    
    # Test zero float
    parser3 = Parser(float)
    xml_data3 = '''<value type="float">0.0</value>'''
    
    result3 = parser3.feed(xml_data3)
    assert result3 == 0.0
    assert isinstance(result3, float)
    
    # Test string with special characters
    parser4 = Parser(str)
    xml_data4 = '''<value type="string">Special &lt;chars&gt; &amp; "quotes"</value>'''
    
    result4 = parser4.feed(xml_data4)
    assert result4 == 'Special <chars> & "quotes"'
    assert isinstance(result4, str)


def test_type_attribute_variations():
    """Test different type attribute formats"""
    # Test with str instead of string
    parser = Parser(str)
    xml_data = '''<value type="str">Test string</value>'''
    
    result = parser.feed(xml_data)
    assert result == "Test string"
    assert isinstance(result, str)
    
    # Test with integer instead of int
    parser2 = Parser(int)
    xml_data2 = '''<value type="integer">999</value>'''
    
    result2 = parser2.feed(xml_data2)
    assert result2 == 999
    assert isinstance(result2, int)
    
    # Test with boolean instead of bool
    parser3 = Parser(bool)
    xml_data3 = '''<value type="boolean">true</value>'''
    
    result3 = parser3.feed(xml_data3)
    assert result3 is True
    assert isinstance(result3, bool)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
