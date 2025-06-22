"""Debug parser flow for field tags with type attributes"""

import os
os.environ['RUST_LOG'] = 'gasp::parser=debug'

from typing import List, Optional
from gasp import Parser, Deserializable


class TestCase(Deserializable):
    """Test case with optional list field"""
    items: Optional[List[str]]


# Test with type attribute on field tag
xml = """<TestCase>
    <items type="list[str]">
        <item type="str">A</item>
        <item type="str">B</item>
    </items>
</TestCase>"""

print("=== Parsing with type attribute on field ===")
parser = Parser(TestCase)
parser.feed(xml)
result = parser.validate()

if result:
    print(f"\nResult: {result}")
    print(f"Items: {result.items}")
    if result.items:
        print(f"Items length: {len(result.items)}")
else:
    print("No result!")
