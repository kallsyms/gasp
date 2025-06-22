"""Debug dict parsing issue"""

from gasp import Parser, Deserializable
from typing import List, Dict
import os

# Enable debug
os.environ['GASP_DEBUG'] = '1'

class Department(Deserializable):
    name: str
    budget: float

class Employee(Deserializable):
    name: str
    id: int
    email: str
    department: Department

class SimpleCompany(Deserializable):
    name: str
    departments: List[Department]
    employee_directory: dict[str, Employee]

# Test the failing XML
xml = """<SimpleCompany>
    <name>TechCorp</name>
    <departments>
        <item>
            <name>Engineering</name>
            <budget>1000000.0</budget>
        </item>
    </departments>
    <employee_directory>
        <item key="alice" type="Employee">
            <name>Alice</name>
            <id>1</id>
            <email>alice@techcorp.com</email>
            <department>
                <name>Engineering</name>
                <budget>1000000.0</budget>
            </department>
        </item>
    </employee_directory>
</SimpleCompany>"""

print("=== Parsing SimpleCompany ===")
parser = Parser(SimpleCompany)
parser.feed(xml)

# Check state during parsing
partial = parser.get_partial()
print(f"\nPartial object: {partial}")
if partial:
    print(f"Name: {getattr(partial, 'name', 'NOT SET')}")
    print(f"Departments: {getattr(partial, 'departments', 'NOT SET')}")
    print(f"Employee directory: {getattr(partial, 'employee_directory', 'NOT SET')}")

result = parser.validate()
print(f"\nFinal result: {result}")
if result:
    print(f"Name: {result.name}")
    print(f"Departments: {result.departments}")
    print(f"Employee directory: {result.employee_directory}")
    print(f"Employee directory keys: {list(result.employee_directory.keys())}")

# Test a simpler dict case
print("\n\n=== Testing simple dict[str, str] ===")

class SimpleDict(Deserializable):
    data: dict[str, str]

xml2 = """<SimpleDict>
    <data>
        <item key="key1">value1</item>
        <item key="key2">value2</item>
    </data>
</SimpleDict>"""

parser2 = Parser(SimpleDict)
parser2.feed(xml2)
result2 = parser2.validate()
print(f"Result: {result2}")
if result2:
    print(f"Data: {result2.data}")
