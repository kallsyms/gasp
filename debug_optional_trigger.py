"""Confirm that Optional is the trigger for the name overwriting bug"""

import os
os.environ['RUST_LOG'] = 'debug'

from typing import List, Optional
from gasp import Parser, Deserializable


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


# Test various optional configurations
class Company1(Deserializable):
    """Required list"""
    name: str
    projects: List[Project]

class Company2(Deserializable):
    """Optional list"""
    name: str
    projects: Optional[List[Project]]

class Company3(Deserializable):
    """Optional single object"""
    name: str
    main_project: Optional[Project]

class Company4(Deserializable):
    """Optional with different nested field name"""
    name: str
    projects: Optional[List[Project]]


# Common XML structure
def make_xml(class_name: str, projects_field: str = "projects", projects_type: str = "list[Project]"):
    return f"""<{class_name}>
    <name type="str">TechCorp</name>
    <{projects_field} type="{projects_type}">
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
        </item>
    </{projects_field}>
</{class_name}>"""


print("=== Test 1: Required List[Project] ===")
parser1 = Parser(Company1)
parser1.feed(make_xml("Company1"))
result1 = parser1.validate()
print(f"Company.name: {result1.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result1.name == 'Engineering'}")

print("\n=== Test 2: Optional[List[Project]] ===")
parser2 = Parser(Company2)
parser2.feed(make_xml("Company2"))
result2 = parser2.validate()
print(f"Company.name: {result2.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result2.name == 'Engineering'}")

print("\n=== Test 3: Optional[Project] (single) ===")
xml3 = """<Company3>
    <name type="str">TechCorp</name>
    <main_project type="Project">
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
    </main_project>
</Company3>"""
parser3 = Parser(Company3)
parser3.feed(xml3)
result3 = parser3.validate()
print(f"Company.name: {result3.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result3.name == 'Engineering'}")

# Test with a class where Employee doesn't have department
class SimpleEmployee(Deserializable):
    name: str
    id: int

class SimpleProject(Deserializable):
    title: str
    team_members: List[SimpleEmployee]

class Company5(Deserializable):
    name: str
    projects: Optional[List[SimpleProject]]

print("\n=== Test 4: Optional[List[SimpleProject]] (shallower nesting) ===")
xml4 = """<Company5>
    <name type="str">TechCorp</name>
    <projects type="list[SimpleProject]">
        <item type="SimpleProject">
            <title type="str">AI Platform</title>
            <team_members type="list[SimpleEmployee]">
                <item type="SimpleEmployee">
                    <name type="str">Alice</name>
                    <id type="int">1</id>
                </item>
            </team_members>
        </item>
    </projects>
</Company5>"""
parser4 = Parser(Company5)
parser4.feed(xml4)
result4 = parser4.validate()
print(f"Company.name: {result4.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result4.name == 'Alice'}")

# Test different field ordering
print("\n=== Test 5: Optional[List[Project]] with name after ===")
xml5 = """<Company2>
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
        </item>
    </projects>
    <name type="str">TechCorp</name>
</Company2>"""
parser5 = Parser(Company2)
parser5.feed(xml5)
result5 = parser5.validate()
print(f"Company.name: {result5.name} (expected: TechCorp)")
print(f"BUG PRESENT: {result5.name == 'Engineering'}")
print(f"Note: Name correctly set when it comes after optional field")
