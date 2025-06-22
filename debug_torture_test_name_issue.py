"""Debug script to trace the name field issue in torture test"""

import os
os.environ['GASP_DEBUG'] = '1'

from typing import List, Union, Optional
from gasp import Parser, Deserializable


class Department(Deserializable):
    """A department with nested structure"""
    name: str
    budget: float
    
    
class Employee(Deserializable):
    """An employee with various attributes"""
    name: str
    id: int
    email: str
    department: Department
    

class Project(Deserializable):
    """A project with complex nested structure"""
    title: str
    description: str
    team_members: List[Employee]
    milestones: dict[str, str]
    

class Company(Deserializable):
    """Root level company with extreme nesting"""
    name: str
    # Mix of optional and non-optional containers
    departments: List[Department]  # Required list
    projects: Optional[List[Project]]  # Optional list of projects
    employee_directory: dict[str, Employee]  # Required dict
    # Deeply nested: optional dict of lists of tuples
    regional_data: Optional[dict[str, List[tuple[str, int, Optional[float]]]]]
    # Set of tags - optional
    company_tags: Optional[set[str]]
    # Complex union in a list
    announcements: List[Union[str, dict[str, Union[str, int]]]]
    # Nested optional lists
    quarterly_reports: Optional[List[Optional[dict[str, Union[float, List[str]]]]]]
    # Tuple with mixed types including optional containers
    metadata: tuple[str, int, Optional[List[dict[str, Employee]]]]
    # Super nested: List of dicts containing optional lists of unions
    initiatives: List[dict[str, Optional[List[Union[Project, Department]]]]]


# First, let's test a minimal example to isolate the issue
xml_minimal = """<Company>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Engineering</name>
            <budget type="float">1000000.0</budget>
        </item>
    </departments>
    <employee_directory type="dict[str, Employee]" />
    <announcements type="list[Union[str, dict[str, Union[str, int]]]]" />
    <metadata type="tuple[str, int, Union[list[dict[str, Employee]], None]]">
        <item type="str">Meta</item>
        <item type="int">2024</item>
        <item type="None">None</item>
    </metadata>
    <initiatives type="list[dict[str, Union[list[Union[Project, Department]], None]]]" />
</Company>"""

print("=== Testing minimal example ===")
parser = Parser(Company)
parser.feed(xml_minimal)

# Check partial state
partial = parser.get_partial()
if partial:
    print(f"Partial Company name: {getattr(partial, 'name', 'NOT SET')}")
    if hasattr(partial, 'departments') and partial.departments:
        print(f"First department name: {partial.departments[0].name if partial.departments else 'NO DEPTS'}")

result = parser.validate()
print(f"\nFinal Company name: {result.name}")
print(f"Departments: {[d.name for d in result.departments]}")


# Now let's trace exactly when the name gets overwritten
print("\n\n=== Testing with incremental feeding ===")

class TracingCompany(Deserializable):
    """Company that traces name changes"""
    name: str
    departments: List[Department]
    
    def __setattr__(self, key, value):
        if key == 'name':
            print(f"TracingCompany.__setattr__: Setting name to '{value}'")
            import traceback
            traceback.print_stack()
        super().__setattr__(key, value)


xml_trace = """<TracingCompany>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Engineering</name>
            <budget type="float">1000000.0</budget>
        </item>
    </departments>
</TracingCompany>"""

parser2 = Parser(TracingCompany)

# Feed line by line to see when it happens
lines = xml_trace.strip().split('\n')
for i, line in enumerate(lines):
    print(f"\nFeeding line {i}: {line}")
    parser2.feed(line + '\n')
    partial = parser2.get_partial()
    if partial:
        print(f"  Company name after this line: {getattr(partial, 'name', 'NOT SET')}")
