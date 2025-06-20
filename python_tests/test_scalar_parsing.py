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
    xml_data = '<str>Hello, world!</str>'
    
    result = parser.feed(xml_data)
    assert result is not None
    assert result == "Hello, world!"
    assert isinstance(result, str)
    assert parser.is_complete()
    
    # Test another string
    parser2 = Parser(str)
    xml_data2 = '<str>Simple string</str>'
    
    result2 = parser2.feed(xml_data2)
    assert result2 is not None
    assert result2 == "Simple string"
    assert isinstance(result2, str)
    assert parser2.is_complete()


def test_int_parsing():
    """Test integer parsing"""
    # Test with XML tags
    parser = Parser(int)
    xml_data = '<int>42</int>'
    
    result = parser.feed(xml_data)
    assert result is not None
    assert result == 42
    assert isinstance(result, int)
    assert parser.is_complete()
    
    # Test another integer
    parser2 = Parser(int)
    xml_data2 = '<int>123</int>'
    
    result2 = parser2.feed(xml_data2)
    assert result2 is not None
    assert result2 == 123
    assert isinstance(result2, int)
    assert parser2.is_complete()
    
    # Test negative integer
    parser3 = Parser(int)
    xml_data3 = '<int>-456</int>'
    
    result3 = parser3.feed(xml_data3)
    assert result3 is not None
    assert result3 == -456
    assert isinstance(result3, int)


def test_float_parsing():
    """Test float parsing"""
    # Test with XML tags
    parser = Parser(float)
    xml_data = '<float>3.14159</float>'
    
    result = parser.feed(xml_data)
    assert result is not None
    assert abs(result - 3.14159) < 0.0001
    assert isinstance(result, float)
    assert parser.is_complete()
    
    # Test another float
    parser2 = Parser(float)
    xml_data2 = '<float>2.71828</float>'
    
    result2 = parser2.feed(xml_data2)
    assert result2 is not None
    assert abs(result2 - 2.71828) < 0.0001
    assert isinstance(result2, float)
    assert parser2.is_complete()
    
    # Test negative float
    parser3 = Parser(float)
    xml_data3 = '<float>-1.5</float>'
    
    result3 = parser3.feed(xml_data3)
    assert result3 is not None
    assert result3 == -1.5
    assert isinstance(result3, float)


def test_bool_parsing():
    """Test boolean parsing"""
    # Test True with XML tags
    parser = Parser(bool)
    xml_data = '<bool>true</bool>'
    
    result = parser.feed(xml_data)
    assert result is not None
    assert result is True
    assert isinstance(result, bool)
    assert parser.is_complete()
    
    # Test False
    parser2 = Parser(bool)
    xml_data2 = '<bool>false</bool>'
    
    result2 = parser2.feed(xml_data2)
    assert result2 is not None
    assert result2 is False
    assert isinstance(result2, bool)
    assert parser2.is_complete()
    
    # Test with different case (should still work)
    parser3 = Parser(bool)
    xml_data3 = '<bool>True</bool>'
    
    result3 = parser3.feed(xml_data3)
    assert result3 is not None
    assert result3 is True
    assert isinstance(result3, bool)
    
    # Test False with different case
    parser4 = Parser(bool)
    xml_data4 = '<bool>False</bool>'
    
    result4 = parser4.feed(xml_data4)
    assert result4 is not None
    assert result4 is False
    assert isinstance(result4, bool)
    
    # Test with 1 and 0
    parser5 = Parser(bool)
    xml_data5 = '<bool>1</bool>'
    
    result5 = parser5.feed(xml_data5)
    assert result5 is not None
    assert result5 is True
    assert isinstance(result5, bool)
    
    parser6 = Parser(bool)
    xml_data6 = '<bool>0</bool>'
    
    result6 = parser6.feed(xml_data6)
    assert result6 is not None
    assert result6 is False
    assert isinstance(result6, bool)


def test_none_parsing():
    """Test null/None parsing"""
    # Python doesn't have a direct None type class, so we skip this for now
    pass


def test_streaming_scalar_parsing():
    """Test streaming parsing of scalar types"""
    # Test streaming string
    parser = Parser(str)
    chunks = [
        '<str>',
        'Hello, ',
        'streaming ',
        'world!',
        '</str>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert result == "Hello, streaming world!"
    assert parser.is_complete()
    
    # Test streaming integer
    parser2 = Parser(int)
    chunks2 = [
        '<int>',
        '12345',
        '</int>'
    ]
    
    result2 = None
    for chunk in chunks2:
        result2 = parser2.feed(chunk)
    
    assert result2 is not None
    assert result2 == 12345
    assert parser2.is_complete()


def test_edge_cases():
    """Test edge cases for scalar parsing"""
    # Test empty string
    parser = Parser(str)
    xml_data = '<str></str>'
    
    result = parser.feed(xml_data)
    assert result is not None
    assert result == ""
    assert isinstance(result, str)
    
    # Test zero integer
    parser2 = Parser(int)
    xml_data2 = '<int>0</int>'
    
    result2 = parser2.feed(xml_data2)
    assert result2 is not None
    assert result2 == 0
    assert isinstance(result2, int)
    
    # Test zero float
    parser3 = Parser(float)
    xml_data3 = '<float>0.0</float>'
    
    result3 = parser3.feed(xml_data3)
    assert result3 is not None
    assert result3 == 0.0
    assert isinstance(result3, float)
    
    # Test string with special characters
    parser4 = Parser(str)
    xml_data4 = '<str>Special &lt;chars&gt; &amp; "quotes"</str>'
    
    result4 = parser4.feed(xml_data4)
    assert result4 is not None
    assert result4 == 'Special <chars> & "quotes"'
    assert isinstance(result4, str)


def test_multiple_scalar_types():
    """Test various scalar type patterns"""
    # Test string with numbers
    parser = Parser(str)
    xml_data = '<str>123abc</str>'
    
    result = parser.feed(xml_data)
    assert result == "123abc"
    assert isinstance(result, str)
    
    # Test float with no decimal
    parser2 = Parser(float)
    xml_data2 = '<float>5</float>'
    
    result2 = parser2.feed(xml_data2)
    assert result2 == 5.0
    assert isinstance(result2, float)
    
    # Test scientific notation
    parser3 = Parser(float)
    xml_data3 = '<float>1.23e-4</float>'
    
    result3 = parser3.feed(xml_data3)
    assert abs(result3 - 0.000123) < 0.0000001
    assert isinstance(result3, float)


def test_invalid_parsing():
    """Test parsing invalid values"""
    # Invalid integer should return None
    parser = Parser(int)
    xml_data = '<int>not_a_number</int>'
    
    result = parser.feed(xml_data)
    assert result is None
    
    # Invalid float should return None
    parser2 = Parser(float)
    xml_data2 = '<float>not_a_float</float>'
    
    result2 = parser2.feed(xml_data2)
    assert result2 is None
    
    # Invalid boolean should return False
    parser3 = Parser(bool)
    xml_data3 = '<bool>maybe</bool>'
    
    result3 = parser3.feed(xml_data3)
    assert result3 is False  # Default to False for invalid values


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
