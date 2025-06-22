"""Find the exact depth/structure that triggers the name overwriting bug"""

import os
os.environ['RUST_LOG'] = 'debug'

from typing import List, Optional
from gasp import Parser, Deserializable


# Test 1: Match the exact torture test structure
class Department(Deserializable):
    name: str
    budget: float


class Employee(Deserializable):
    name: str
    id: int
    email: str
    department: Department


class Project(Deserializable):
    title: str
    description: str
    team_members: List[Employee]
    milestones: dict[str, str]


class CompanySimple(Deserializable):
    name: str
    projects: List[Project]


print("=== Test 1: Minimal Company with projects ===")
xml1 = """<CompanySimple>
    <name type="str">TechCorp</name>
    <projects type="list[Project]">
        <item type="Project">
            <title type="str">AI Platform</title>
            <description type="str">Next-gen AI platform</description>
            <team_members type="list[Employee]">
                <item type="Employee">
                    <name type="str">Alice</name>
                    <id type="int">1</id>
                    <email type="str">alice@techcorp.com</email>
                    <department type="Department">
                        <name type="str">Engineering</name>
                        <budget type="float">1000000.0</budget>
                    </department>
                </item>
            </team_members>
            <milestones type="dict[str, str]">
                <item key="Q1" type="str">Planning</item>
            </milestones>
        </item>
    </projects>
</CompanySimple>"""

parser1 = Parser(CompanySimple)
parser1.feed(xml1)
result1 = parser1.validate()
print(f"Company.name: {result1.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result1.name == 'Engineering'}")

# Test 2: Add more fields like in torture test
class CompanyWithMore(Deserializable):
    name: str
    departments: List[Department]
    projects: List[Project]
    employee_directory: dict[str, Employee]

print("\n=== Test 2: Company with more fields ===")
xml2 = """<CompanyWithMore>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Sales</name>
            <budget type="float">500000.0</budget>
        </item>
    </departments>
    <projects type="list[Project]">
        <item type="Project">
            <title type="str">AI Platform</title>
            <description type="str">Next-gen AI platform</description>
            <team_members type="list[Employee]">
                <item type="Employee">
                    <name type="str">Alice</name>
                    <id type="int">1</id>
                    <email type="str">alice@techcorp.com</email>
                    <department type="Department">
                        <name type="str">Engineering</name>
                        <budget type="float">1000000.0</budget>
                    </department>
                </item>
            </team_members>
            <milestones type="dict[str, str]">
                <item key="Q1" type="str">Planning</item>
            </milestones>
        </item>
    </projects>
    <employee_directory type="dict[str, Employee]">
        <item key="alice" type="Employee">
            <name type="str">Alice</name>
            <id type="int">1</id>
            <email type="str">alice@techcorp.com</email>
            <department type="Department">
                <name type="str">Engineering</name>
                <budget type="float">1000000.0</budget>
            </department>
        </item>
    </employee_directory>
</CompanyWithMore>"""

parser2 = Parser(CompanyWithMore)
parser2.feed(xml2)
result2 = parser2.validate()
print(f"Company.name: {result2.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result2.name == 'Engineering'}")

# Test 3: With optional projects like in torture test
class CompanyOptional(Deserializable):
    name: str
    departments: List[Department]
    projects: Optional[List[Project]]
    employee_directory: dict[str, Employee]

print("\n=== Test 3: Company with optional projects ===")
xml3 = """<CompanyOptional>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Sales</name>
            <budget type="float">500000.0</budget>
        </item>
    </departments>
    <projects type="list[Project]">
        <item type="Project">
            <title type="str">AI Platform</title>
            <description type="str">Next-gen AI platform</description>
            <team_members type="list[Employee]">
                <item type="Employee">
                    <name type="str">Alice</name>
                    <id type="int">1</id>
                    <email type="str">alice@techcorp.com</email>
                    <department type="Department">
                        <name type="str">Engineering</name>
                        <budget type="float">1000000.0</budget>
                    </department>
                </item>
            </team_members>
            <milestones type="dict[str, str]">
                <item key="Q1" type="str">Planning</item>
            </milestones>
        </item>
    </projects>
    <employee_directory type="dict[str, Employee]">
        <item key="alice" type="Employee">
            <name type="str">Alice</name>
            <id type="int">1</id>
            <email type="str">alice@techcorp.com</email>
            <department type="Department">
                <name type="str">Engineering</name>
                <budget type="float">1000000.0</budget>
            </department>
        </item>
    </employee_directory>
</CompanyOptional>"""

parser3 = Parser(CompanyOptional)
parser3.feed(xml3)
result3 = parser3.validate()
print(f"Company.name: {result3.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result3.name == 'Engineering'}")

# Test 4: Check if it's about the field ordering
print("\n=== Test 4: Projects before name ===")
xml4 = """<CompanyOptional>
    <projects type="list[Project]">
        <item type="Project">
            <title type="str">AI Platform</title>
            <description type="str">Next-gen AI platform</description>
            <team_members type="list[Employee]">
                <item type="Employee">
                    <name type="str">Alice</name>
                    <id type="int">1</id>
                    <email type="str">alice@techcorp.com</email>
                    <department type="Department">
                        <name type="str">Engineering</name>
                        <budget type="float">1000000.0</budget>
                    </department>
                </item>
            </team_members>
            <milestones type="dict[str, str]">
                <item key="Q1" type="str">Planning</item>
            </milestones>
        </item>
    </projects>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Sales</name>
            <budget type="float">500000.0</budget>
        </item>
    </departments>
    <employee_directory type="dict[str, Employee]">
        <item key="alice" type="Employee">
            <name type="str">Alice</name>
            <id type="int">1</id>
            <email type="str">alice@techcorp.com</email>
            <department type="Department">
                <name type="str">Engineering</name>
                <budget type="float">1000000.0</budget>
            </department>
        </item>
    </employee_directory>
</CompanyOptional>"""

parser4 = Parser(CompanyOptional)
parser4.feed(xml4)
result4 = parser4.validate()
print(f"Company.name: {result4.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result4.name == 'Engineering'}")
print(f"Note: When projects comes first, name is: {result4.name}")
