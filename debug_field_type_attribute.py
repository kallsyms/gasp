"""Debug how parser handles field tags with complex type attributes"""

import os
os.environ['GASP_DEBUG'] = '1'

from typing import List, Union, Optional
from gasp import Parser, Deserializable


class SimpleCase(Deserializable):
    """Test with a simpler field to isolate the issue"""
    items: List[str]


# First test: simple case with type attribute on field
xml1 = """<SimpleCase>
    <items type="list[str]">
        <item type="str">A</item>
        <item type="str">B</item>
    </items>
</SimpleCase>"""

print("=== Test 1: Simple field with type attribute ===")
parser1 = Parser(SimpleCase)
parser1.feed(xml1)
result1 = parser1.validate()
print(f"Result: {result1}")
if result1:
    print(f"Items: {result1.items}")
    print(f"Items length: {len(result1.items)}")


# Now test the problematic case
class ComplexCase(Deserializable):
    """Test with complex nested type"""
    quarterly_reports: List[Optional[dict[str, Union[float, List[str]]]]]


xml2 = """<ComplexCase>
    <quarterly_reports type="list[Union[dict[str, Union[float, list[str]]], None]]">
        <item type="dict[str, Union[float, list[str]]]">
            <item key="revenue" type="float">1500000.0</item>
            <item key="highlights" type="list[str]">
                <item type="str">New product launch</item>
                <item type="str">Exceeded targets</item>
            </item>
        </item>
        <item type="None">None</item>
    </quarterly_reports>
</ComplexCase>"""

print("\n\n=== Test 2: Complex field with type attribute ===")
parser2 = Parser(ComplexCase)
parser2.feed(xml2)
result2 = parser2.validate()
print(f"Result: {result2}")
if result2:
    print(f"Quarterly reports: {result2.quarterly_reports}")
    if result2.quarterly_reports:
        print(f"Length: {len(result2.quarterly_reports)}")


# Test 3: Same structure but without type attribute on field
xml3 = """<ComplexCase>
    <quarterly_reports>
        <item type="dict[str, Union[float, list[str]]]">
            <item key="revenue" type="float">1500000.0</item>
        </item>
    </quarterly_reports>
</ComplexCase>"""

print("\n\n=== Test 3: Without type attribute on field ===")
parser3 = Parser(ComplexCase)
parser3.feed(xml3)
result3 = parser3.validate()
print(f"Result: {result3}")
if result3:
    print(f"Quarterly reports: {result3.quarterly_reports}")
    if result3.quarterly_reports:
        print(f"Length: {len(result3.quarterly_reports)}")
