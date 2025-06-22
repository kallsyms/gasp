"""Debug quarterly_reports parsing issue"""

import os
os.environ['GASP_DEBUG'] = '1'

from typing import List, Union, Optional
from gasp import Parser, Deserializable


class Company(Deserializable):
    """Simplified company with just the problematic field"""
    name: str
    quarterly_reports: Optional[List[Optional[dict[str, Union[float, List[str]]]]]]


# Test with the exact XML structure from the test
xml = """<Company>
    <name type="str">TechCorp</name>
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
</Company>"""

print("=== Parsing quarterly_reports field ===")
parser = Parser(Company)
parser.feed(xml)

# Check partial state
partial = parser.get_partial()
print(f"\nPartial object: {partial}")
if partial:
    print(f"Name: {getattr(partial, 'name', 'NOT SET')}")
    print(f"Quarterly reports: {getattr(partial, 'quarterly_reports', 'NOT SET')}")
    if hasattr(partial, 'quarterly_reports') and partial.quarterly_reports is not None:
        print(f"  Length: {len(partial.quarterly_reports)}")
        for i, report in enumerate(partial.quarterly_reports):
            print(f"  Item {i}: {report}")

result = parser.validate()
print(f"\nValidated result: {result}")
if result:
    print(f"Name: {result.name}")
    print(f"Quarterly reports: {result.quarterly_reports}")
    if result.quarterly_reports:
        print(f"  Length: {len(result.quarterly_reports)}")
        for i, report in enumerate(result.quarterly_reports):
            print(f"  Item {i}: {report}")

# Now test without the type attribute on the field tag
xml2 = """<Company>
    <name type="str">TechCorp</name>
    <quarterly_reports>
        <item type="dict[str, Union[float, list[str]]]">
            <item key="revenue" type="float">1500000.0</item>
            <item key="highlights" type="list[str]">
                <item type="str">New product launch</item>
                <item type="str">Exceeded targets</item>
            </item>
        </item>
        <item type="None">None</item>
    </quarterly_reports>
</Company>"""

print("\n\n=== Parsing without type attribute on field tag ===")
parser2 = Parser(Company)
parser2.feed(xml2)
result2 = parser2.validate()
print(f"\nValidated result: {result2}")
if result2:
    print(f"Name: {result2.name}")
    print(f"Quarterly reports: {result2.quarterly_reports}")
    if result2.quarterly_reports:
        print(f"  Length: {len(result2.quarterly_reports)}")
        for i, report in enumerate(result2.quarterly_reports):
            print(f"  Item {i}: {report}")
