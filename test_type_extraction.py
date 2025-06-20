#!/usr/bin/env python3
"""Test type extraction for debugging"""

from typing import Dict, Set, Tuple, List
import gasp

# Let's see what the parser does with these types
print("Testing type extraction...")

# Test Dict[str, int]
try:
    parser = gasp.Parser(Dict[str, int])
    print(f"Dict[str, int] parser created successfully")
    
    # Try feeding it XML
    result = parser.feed('<dict><item key="a">1</item></dict>')
    print(f"Result with <dict> tag: {result}")
    
    # Try with different tag names
    parser2 = gasp.Parser(Dict[str, int])
    result2 = parser2.feed('<Dict><item key="a">1</item></Dict>')
    print(f"Result with <Dict> tag: {result2}")
    
except Exception as e:
    print(f"Error: {e}")

# Test plain dict
try:
    parser3 = gasp.Parser(dict)
    print(f"\ndict parser created successfully")
    result3 = parser3.feed('<dict><item key="a">1</item></dict>')
    print(f"Result with <dict> tag: {result3}")
except Exception as e:
    print(f"Error: {e}")
