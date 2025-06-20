#!/usr/bin/env python3
"""Debug scalar type wanted tags configuration"""

from gasp import Parser
import logging

# Set up logging to see debug output
logging.basicConfig(level=logging.DEBUG)

def test_str_parser_config():
    """Test how Parser is configured for str type"""
    print("Creating Parser for str type...")
    parser = Parser(str)
    
    # Try to access internal state (if accessible)
    print(f"Parser object: {parser}")
    print(f"Parser type: {type(parser)}")
    
    # Test with simple input
    xml = '<str>Hello</str>'
    print(f"\nFeeding: {repr(xml)}")
    result = parser.feed(xml)
    print(f"Result: {result}")
    print(f"Complete: {parser.is_complete()}")

if __name__ == "__main__":
    test_str_parser_config()
