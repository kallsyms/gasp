"""Test if the bug affects other optional container types"""

import os
os.environ['RUST_LOG'] = 'debug'

from typing import List, Optional, Dict, Set, Tuple
from gasp import Parser, Deserializable


class Department(Deserializable):
    name: str
    budget: float


class Employee(Deserializable):
    name: str
    id: int  
    email: str
    department: Department


# Test Optional[Dict[str, T]]
class CompanyDict(Deserializable):
    name: str
    employee_map: Optional[Dict[str, Employee]]


print("=== Test 1: Optional[Dict[str, Employee]] ===")
xml1 = """<CompanyDict>
    <name type="str">TechCorp</name>
    <employee_map type="dict[str, Employee]">
        <item key="alice" type="Employee">
            <name type="str">Alice</name>
            <id type="int">1</id>
            <email type="str">alice@techcorp.com</email>
            <department type="Department">
                <name type="str">Engineering</name>
                <budget type="float">1000000.0</budget>
            </department>
        </item>
    </employee_map>
</CompanyDict>"""

parser1 = Parser(CompanyDict)
parser1.feed(xml1)
result1 = parser1.validate()
print(f"Company.name: {result1.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result1.name != 'TechCorp'}")
print(f"Overwritten with: {result1.name}")


# Test Optional[Set[T]] with objects that have name
class NamedItem(Deserializable):
    name: str
    value: int


class CompanySet(Deserializable):
    name: str
    unique_items: Optional[Set[NamedItem]]


print("\n=== Test 2: Optional[Set[NamedItem]] ===")
xml2 = """<CompanySet>
    <name type="str">TechCorp</name>
    <unique_items type="set[NamedItem]">
        <item type="NamedItem">
            <name type="str">ItemAlpha</name>
            <value type="int">100</value>
        </item>
        <item type="NamedItem">
            <name type="str">ItemBeta</name>
            <value type="int">200</value>
        </item>
    </unique_items>
</CompanySet>"""

parser2 = Parser(CompanySet)
parser2.feed(xml2)
result2 = parser2.validate()
print(f"Company.name: {result2.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result2.name != 'TechCorp'}")
print(f"Overwritten with: {result2.name}")


# Test Optional[Tuple[...]] with nested Employee
class CompanyTuple(Deserializable):
    name: str
    key_employees: Optional[Tuple[Employee, Employee]]


print("\n=== Test 3: Optional[Tuple[Employee, Employee]] ===")
xml3 = """<CompanyTuple>
    <name type="str">TechCorp</name>
    <key_employees type="tuple[Employee, Employee]">
        <item type="Employee">
            <name type="str">Alice</name>
            <id type="int">1</id>
            <email type="str">alice@techcorp.com</email>
            <department type="Department">
                <name type="str">Engineering</name>
                <budget type="float">1000000.0</budget>
            </department>
        </item>
        <item type="Employee">
            <name type="str">Bob</name>
            <id type="int">2</id>
            <email type="str">bob@techcorp.com</email>
            <department type="Department">
                <name type="str">Sales</name>
                <budget type="float">500000.0</budget>
            </department>
        </item>
    </key_employees>
</CompanyTuple>"""

parser3 = Parser(CompanyTuple)
parser3.feed(xml3)
result3 = parser3.validate()
print(f"Company.name: {result3.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result3.name != 'TechCorp'}")
print(f"Overwritten with: {result3.name}")


# Test required versions for comparison
class CompanyDictRequired(Deserializable):
    name: str
    employee_map: Dict[str, Employee]  # Required, not optional


print("\n=== Test 4: Required Dict[str, Employee] (for comparison) ===")
parser4 = Parser(CompanyDictRequired)
parser4.feed(xml1.replace("CompanyDict", "CompanyDictRequired"))
result4 = parser4.validate()
print(f"Company.name: {result4.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result4.name != 'TechCorp'}")


# Test with field order reversed for Optional[Dict]
print("\n=== Test 5: Optional[Dict] with name after ===")
xml5 = """<CompanyDict>
    <employee_map type="dict[str, Employee]">
        <item key="alice" type="Employee">
            <name type="str">Alice</name>
            <id type="int">1</id>
            <email type="str">alice@techcorp.com</email>
            <department type="Department">
                <name type="str">Engineering</name>
                <budget type="float">1000000.0</budget>
            </department>
        </item>
    </employee_map>
    <name type="str">TechCorp</name>
</CompanyDict>"""

parser5 = Parser(CompanyDict)
parser5.feed(xml5)
result5 = parser5.validate()
print(f"Company.name: {result5.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result5.name != 'TechCorp'}")
print(f"Note: Name correctly set when it comes after optional field")


# Test nested optional containers
class CompanyNested(Deserializable):
    name: str
    dept_map: Optional[Dict[str, List[Employee]]]


print("\n=== Test 6: Optional[Dict[str, List[Employee]]] (nested containers) ===")
xml6 = """<CompanyNested>
    <name type="str">TechCorp</name>
    <dept_map type="dict[str, list[Employee]]">
        <item key="engineering" type="list[Employee]">
            <item type="Employee">
                <name type="str">Alice</name>
                <id type="int">1</id>
                <email type="str">alice@techcorp.com</email>
                <department type="Department">
                    <name type="str">Engineering</name>
                    <budget type="float">1000000.0</budget>
                </department>
            </item>
        </item>
    </dept_map>
</CompanyNested>"""

parser6 = Parser(CompanyNested)
parser6.feed(xml6)
result6 = parser6.validate()
print(f"Company.name: {result6.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result6.name != 'TechCorp'}")
print(f"Overwritten with: {result6.name}")
