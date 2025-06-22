"""Minimal test to confirm the nested name overwriting bug"""

import os
os.environ['RUST_LOG'] = 'debug'

from typing import List, Optional
from gasp import Parser, Deserializable


class Inner(Deserializable):
    """Inner class with name field"""
    name: str
    

class Middle(Deserializable):
    """Middle class containing Inner"""
    inner: Inner
    

class Outer(Deserializable):
    """Outer class with name and list of Middle"""
    name: str
    middles: List[Middle]


print("=== Test 1: Simple nested name ===")
xml1 = """<Outer>
    <name type="str">OuterName</name>
    <middles type="list[Middle]">
        <item type="Middle">
            <inner type="Inner">
                <name type="str">InnerName</name>
            </inner>
        </item>
    </middles>
</Outer>"""

parser1 = Parser(Outer)
parser1.feed(xml1)
result1 = parser1.validate()
print(f"Outer.name: {result1.name} (expected: OuterName)")
print(f"Inner.name: {result1.middles[0].inner.name} (expected: InnerName)")
print(f"BUG PRESENT: {result1.name == 'InnerName'}")

print("\n=== Test 2: Without the list ===")
class SimpleOuter(Deserializable):
    """Outer with direct middle, no list"""
    name: str
    middle: Middle

xml2 = """<SimpleOuter>
    <name type="str">OuterName</name>
    <middle type="Middle">
        <inner type="Inner">
            <name type="str">InnerName</name>
        </inner>
    </middle>
</SimpleOuter>"""

parser2 = Parser(SimpleOuter)
parser2.feed(xml2)
result2 = parser2.validate()
print(f"Outer.name: {result2.name} (expected: OuterName)")
print(f"Inner.name: {result2.middle.inner.name} (expected: InnerName)")
print(f"BUG PRESENT: {result2.name == 'InnerName'}")

print("\n=== Test 3: Different field names ===")
class InnerWithId(Deserializable):
    """Inner class with id instead of name"""
    id: int
    

class MiddleWithInner(Deserializable):
    """Middle class containing Inner"""
    inner: InnerWithId
    

class OuterWithId(Deserializable):
    """Outer class with id and list of Middle"""
    id: int
    middles: List[MiddleWithInner]

xml3 = """<OuterWithId>
    <id type="int">999</id>
    <middles type="list[MiddleWithInner]">
        <item type="MiddleWithInner">
            <inner type="InnerWithId">
                <id type="int">111</id>
            </inner>
        </item>
    </middles>
</OuterWithId>"""

parser3 = Parser(OuterWithId)
parser3.feed(xml3)
result3 = parser3.validate()
print(f"Outer.id: {result3.id} (expected: 999)")
print(f"Inner.id: {result3.middles[0].inner.id} (expected: 111)")
print(f"BUG PRESENT: {result3.id == 111}")

print("\n=== Test 4: Field order matters? ===")
xml4 = """<Outer>
    <middles type="list[Middle]">
        <item type="Middle">
            <inner type="Inner">
                <name type="str">InnerName</name>
            </inner>
        </item>
    </middles>
    <name type="str">OuterName</name>
</Outer>"""

parser4 = Parser(Outer)
parser4.feed(xml4)
result4 = parser4.validate()
print(f"Outer.name: {result4.name} (expected: OuterName)")
print(f"Inner.name: {result4.middles[0].inner.name} (expected: InnerName)")
print(f"BUG PRESENT: {result4.name == 'InnerName'}")
print(f"Note: With field order reversed, name is: {result4.name}")
