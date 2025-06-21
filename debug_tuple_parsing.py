#!/usr/bin/env python3
"""Debug tuple parsing issue"""

from gasp import Parser
from typing import Tuple

print("=== Testing basic tuple parsing ===")
parser = Parser(tuple)
xml = '''<tuple type="tuple">
    <item type="int">1</item>
    <item type="int">2</item>
    <item type="int">3</item>
</tuple>'''

result = parser.feed(xml)
print(f"Result: {result}")
print(f"Type: {type(result)}")
print(f"Is complete: {parser.is_complete()}")

print("\n=== Testing typed tuple parsing ===")
parser2 = Parser(Tuple[int, int, int])
result2 = parser2.feed(xml)
print(f"Result: {result2}")
print(f"Type: {type(result2)}")
print(f"Is complete: {parser2.is_complete()}")
