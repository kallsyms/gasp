#!/usr/bin/env python3
"""Debug nested list parsing"""

from gasp import Parser
from typing import List

# Test nested lists
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

# Feed in chunks to see incremental parsing
chunks = xml_data.split('\n')
for i, chunk in enumerate(chunks):
    result = parser.feed(chunk + '\n')
    print(f"Chunk {i}: {repr(chunk)}")
    if result is not None:
        print(f"  Result: {result}")
        print(f"  Type: {type(result)}")
        if isinstance(result, list):
            print(f"  Length: {len(result)}")
            for j, item in enumerate(result):
                print(f"    Item {j}: {item} (type: {type(item)})")

print("\nFinal validation:")
final = parser.validate()
print(f"Final result: {final}")
print(f"Is complete: {parser.is_complete()}")
