#!/usr/bin/env python3
"""Debug scalar type streaming"""

from gasp import Parser

def test_streaming_string():
    """Test streaming parsing of string type"""
    parser = Parser(str)
    chunks = [
        '<str>',
        'Hello, ',
        'streaming ',
        'world!',
        '</str>'
    ]
    
    print("Testing streaming string parsing...")
    result = None
    for i, chunk in enumerate(chunks):
        print(f"Feeding chunk {i}: {repr(chunk)}")
        result = parser.feed(chunk)
        print(f"  Result after chunk {i}: {result}")
        print(f"  Parser complete: {parser.is_complete()}")
    
    print(f"\nFinal result: {result}")
    print(f"Final parser complete: {parser.is_complete()}")
    return result

def test_simple_string():
    """Test simple string parsing"""
    parser = Parser(str)
    xml_data = '<str>Hello, world!</str>'
    
    print("\nTesting simple string parsing...")
    print(f"Feeding: {repr(xml_data)}")
    result = parser.feed(xml_data)
    print(f"Result: {result}")
    print(f"Parser complete: {parser.is_complete()}")
    return result

if __name__ == "__main__":
    # Test both cases
    simple_result = test_simple_string()
    streaming_result = test_streaming_string()
    
    print(f"\nSimple result: {simple_result}")
    print(f"Streaming result: {streaming_result}")
