#!/usr/bin/env python3
"""Debug incremental list parsing to understand when items get populated"""

from gasp import Deserializable, Parser
from typing import List


class Item(Deserializable):
    step: int
    title: str
    description: str
    
    @classmethod
    def __gasp_from_partial__(cls, partial_data):
        print(f"\nItem.__gasp_from_partial__ called with keys: {list(partial_data.keys())}")
        instance = super().__gasp_from_partial__(partial_data)
        print(f"  Created Item: step={getattr(instance, 'step', None)}, title={getattr(instance, 'title', None)}")
        return instance
    
    def __gasp_update__(self, updates):
        print(f"\nItem.__gasp_update__ called with updates: {updates}")
        super().__gasp_update__(updates)
        print(f"  After update: step={getattr(self, 'step', None)}, title={getattr(self, 'title', None)}")


# Generate XML for a simple list
xml = """<list type="list[Item]">
<item type="Item">
  <step type="int">1</step>
  <title type="str">First Item</title>
  <description type="str">Description 1</description>
</item>
<item type="Item">
  <step type="int">2</step>
  <title type="str">Second Item</title>
  <description type="str">Description 2</description>
</item>
</list>"""

print("XML to parse:")
print(xml)
print("\n" + "="*60 + "\n")

# Parse in small chunks
parser = Parser(List[Item])
chunk_size = 20
chunks = [xml[i:i+chunk_size] for i in range(0, len(xml), chunk_size)]

result = None
for i, chunk in enumerate(chunks):
    print(f"\nChunk {i}: '{chunk}'")
    result = parser.feed(chunk)
    
    if result:
        print(f"  Result after chunk: {len(result)} items")
        for j, item in enumerate(result):
            if isinstance(item, Item):
                print(f"    Item {j}: step={getattr(item, 'step', None)}, title={getattr(item, 'title', None)}")

print("\n" + "="*60 + "\n")
print("Final result:")
if result:
    for i, item in enumerate(result):
        print(f"  Item {i}: step={item.step}, title={item.title}, description={item.description}")
