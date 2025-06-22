"""Debug the exact failure from the test"""

import os
os.environ['RUST_LOG'] = 'debug'

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


# Test the exact failing XML with just the required fields
xml_test = """
    <Company>
        <name type="str">TechCorp</name>
        <departments type="list[Department]">
            <item type="Department">
                <name type="str">Engineering</name>
                <budget type="float">1000000.0</budget>
            </item>
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
                    <item key="Q2" type="str">Development</item>
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
        <regional_data type="dict[str, list[tuple[str, int, Union[float, None]]]]">
            <item key="North America" type="list[tuple[str, int, Union[float, None]]]">
                <item type="tuple[str, int, Union[float, None]]">
                    <item type="str">USA</item>
                    <item type="int">100</item>
                    <item type="float">95.5</item>
                </item>
                <item type="tuple[str, int, Union[float, None]]">
                    <item type="str">Canada</item>
                    <item type="int">50</item>
                    <item type="None">None</item>
                </item>
            </item>
        </regional_data>
        <company_tags type="set[str]">
            <item type="str">tech</item>
            <item type="str">startup</item>
            <item type="str">AI</item>
        </company_tags>
        <announcements type="list[Union[str, dict[str, Union[str, int]]]]">
            <item type="str">Company picnic next Friday!</item>
            <item type="dict[str, Union[str, int]]">
                <item key="type" type="str">funding</item>
                <item key="amount" type="int">10000000</item>
            </item>
        </announcements>
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
        <metadata type="tuple[str, int, Union[list[dict[str, Employee]], None]]">
            <item type="str">TechCorp Metadata</item>
            <item type="int">2024</item>
            <item type="list[dict[str, Employee]]">
                <item type="dict[str, Employee]">
                    <item key="ceo" type="Employee">
                        <name type="str">Bob</name>
                        <id type="int">0</id>
                        <email type="str">bob@techcorp.com</email>
                        <department type="Department">
                            <name type="str">Executive</name>
                            <budget type="float">2000000.0</budget>
                        </department>
                    </item>
                </item>
            </item>
        </metadata>
        <initiatives type="list[dict[str, Union[list[Union[Project, Department]], None]]]">
            <item type="dict[str, Union[list[Union[Project, Department]], None]]">
                <item key="Q1-initiatives" type="list[Union[Project, Department]]">
                    <item type="Project">
                        <title type="str">Mobile App</title>
                        <description type="str">New mobile application</description>
                        <team_members type="list[Employee]" />
                        <milestones type="dict[str, str]" />
                    </item>
                    <item type="Department">
                        <name type="str">Mobile Division</name>
                        <budget type="float">750000.0</budget>
                    </item>
                </item>
            </item>
            <item type="dict[str, Union[list[Union[Project, Department]], None]]">
                <item key="Q2-initiatives" type="None">None</item>
            </item>
        </initiatives>
    </Company>"""

print("=== Testing exact failing case ===")
parser = Parser(Company)
parser.feed(xml_test)

# Check partial state before validation
partial = parser.get_partial()
if partial:
    print(f"\nPartial state before validation:")
    print(f"  Company.name: {getattr(partial, 'name', 'NOT SET')}")
    print(f"  Has departments: {hasattr(partial, 'departments')}")
    if hasattr(partial, 'departments') and partial.departments:
        print(f"  Department names: {[d.name for d in partial.departments]}")

# Validate
result = parser.validate()

print(f"\nFinal result after validation:")
print(f"  result.name: {result.name}")
print(f"  Department names: {[d.name for d in result.departments]}")

# Print the model_dump to match test output
print(f"\nmodel_dump output:")
print(result.model_dump())

# Let's also check the raw dict
print(f"\nDirect attribute access:")
print(f"  result.__dict__['name']: {result.__dict__.get('name', 'NOT IN DICT')}")

# Check if model_dump is doing something weird
print(f"\nDetailed model_dump analysis:")
dump = result.model_dump()
print(f"  type(dump): {type(dump)}")
print(f"  dump['name']: {dump.get('name', 'NOT IN DUMP')}")
print(f"  'name' in dump: {'name' in dump}")
print(f"  dump keys: {list(dump.keys())}")

# Test with projects field to reproduce the issue
print("\n\n=== Testing WITH projects field (should fail) ===")
xml_with_projects = """<Company>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Engineering</name>
            <budget type="float">1000000.0</budget>
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
                <item key="Q2" type="str">Development</item>
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
    <announcements type="list[Union[str, dict[str, Union[str, int]]]]">
        <item type="str">Company picnic next Friday!</item>
    </announcements>
    <metadata type="tuple[str, int, Union[list[dict[str, Employee]], None]]">
        <item type="str">TechCorp Metadata</item>
        <item type="int">2024</item>
        <item type="None">None</item>
    </metadata>
    <initiatives type="list[dict[str, Union[list[Union[Project, Department]], None]]]">
        <item type="dict[str, Union[list[Union[Project, Department]], None]]" />
    </initiatives>
</Company>"""

parser_with_projects = Parser(Company)
parser_with_projects.feed(xml_with_projects)
result_with_projects = parser_with_projects.validate()

print(f"\nResult with projects:")
print(f"  Company.name: {result_with_projects.name}")
print(f"  Expected: TechCorp, Got: {result_with_projects.name}")
print(f"  PASS: {result_with_projects.name == 'TechCorp'}")

# Test without projects field
print("\n\n=== Testing WITHOUT projects field (should pass) ===")
xml_without_projects = """<Company>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Engineering</name>
            <budget type="float">1000000.0</budget>
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
    <announcements type="list[Union[str, dict[str, Union[str, int]]]]">
        <item type="str">Company picnic next Friday!</item>
    </announcements>
    <metadata type="tuple[str, int, Union[list[dict[str, Employee]], None]]">
        <item type="str">TechCorp Metadata</item>
        <item type="int">2024</item>
        <item type="None">None</item>
    </metadata>
    <initiatives type="list[dict[str, Union[list[Union[Project, Department]], None]]]">
        <item type="dict[str, Union[list[Union[Project, Department]], None]]" />
    </initiatives>
</Company>"""

parser_without_projects = Parser(Company)
parser_without_projects.feed(xml_without_projects)
result_without_projects = parser_without_projects.validate()

print(f"\nResult without projects:")
print(f"  Company.name: {result_without_projects.name}")
print(f"  Expected: TechCorp, Got: {result_without_projects.name}")
print(f"  PASS: {result_without_projects.name == 'TechCorp'}")

# Test with a different field name instead of projects
print("\n\n=== Testing with 'programs' instead of 'projects' ===")

# First add programs as an optional field
class CompanyWithPrograms(Deserializable):
    """Root level company with programs instead of projects"""
    name: str
    departments: List[Department]
    programs: Optional[List[Project]]  # Same type but different field name
    employee_directory: dict[str, Employee]
    announcements: List[Union[str, dict[str, Union[str, int]]]]
    metadata: tuple[str, int, Optional[List[dict[str, Employee]]]]
    initiatives: List[dict[str, Optional[List[Union[Project, Department]]]]]

xml_with_programs = """<CompanyWithPrograms>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Engineering</name>
            <budget type="float">1000000.0</budget>
        </item>
    </departments>
    <programs type="list[Project]">
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
                <item key="Q2" type="str">Development</item>
            </milestones>
        </item>
    </programs>
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
    <announcements type="list[Union[str, dict[str, Union[str, int]]]]">
        <item type="str">Company picnic next Friday!</item>
    </announcements>
    <metadata type="tuple[str, int, Union[list[dict[str, Employee]], None]]">
        <item type="str">TechCorp Metadata</item>
        <item type="int">2024</item>
        <item type="None">None</item>
    </metadata>
    <initiatives type="list[dict[str, Union[list[Union[Project, Department]], None]]]">
        <item type="dict[str, Union[list[Union[Project, Department]], None]]" />
    </initiatives>
</CompanyWithPrograms>"""

parser_with_programs = Parser(CompanyWithPrograms)
parser_with_programs.feed(xml_with_programs)
result_with_programs = parser_with_programs.validate()

print(f"\nResult with 'programs' field:")
print(f"  Company.name: {result_with_programs.name}")
print(f"  Expected: TechCorp, Got: {result_with_programs.name}")
print(f"  PASS: {result_with_programs.name == 'TechCorp'}")

# Test minimal case that triggers the issue
print("\n\n=== Testing minimal case with nested name field ===")

xml_minimal_nested = """<Company>
    <name type="str">TechCorp</name>
    <departments type="list[Department]">
        <item type="Department">
            <name type="str">Engineering</name>
            <budget type="float">1000000.0</budget>
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
            <milestones type="dict[str, str]" />
        </item>
    </projects>
    <employee_directory type="dict[str, Employee]" />
    <announcements type="list[Union[str, dict[str, Union[str, int]]]]" />
    <metadata type="tuple[str, int, Union[list[dict[str, Employee]], None]]">
        <item type="str">Meta</item>
        <item type="int">2024</item>
        <item type="None">None</item>
    </metadata>
    <initiatives type="list[dict[str, Union[list[Union[Project, Department]], None]]]" />
</Company>"""

parser_minimal = Parser(Company)
parser_minimal.feed(xml_minimal_nested)
result_minimal = parser_minimal.validate()

print(f"\nMinimal nested result:")
print(f"  Company.name: {result_minimal.name}")
print(f"  Project Employee's Department name: {result_minimal.projects[0].team_members[0].department.name}")
print(f"  Is the Company name the same as nested Department name? {result_minimal.name == result_minimal.projects[0].team_members[0].department.name}")

# Test if it's specifically about "name" field or any field
print("\n\n=== Testing if other fields get overwritten ===")

class DepartmentWithId(Deserializable):
    """Department with id instead of name"""
    id: int
    budget: float

class EmployeeWithDept(Deserializable):
    """Employee with department"""
    name: str
    id: int
    email: str
    department: DepartmentWithId

class ProjectWithEmployees(Deserializable):
    """Project with employees"""
    title: str
    description: str
    team_members: List[EmployeeWithDept]

class CompanyWithId(Deserializable):
    """Company with id field to test overwriting"""
    id: int
    name: str
    departments: List[DepartmentWithId]
    projects: List[ProjectWithEmployees]

xml_test_id = """<CompanyWithId>
    <id type="int">999</id>
    <name type="str">TechCorp</name>
    <departments type="list[DepartmentWithId]">
        <item type="DepartmentWithId">
            <id type="int">100</id>
            <budget type="float">1000000.0</budget>
        </item>
    </departments>
    <projects type="list[ProjectWithEmployees]">
        <item type="ProjectWithEmployees">
            <title type="str">AI Platform</title>
            <description type="str">Next-gen AI platform</description>
            <team_members type="list[EmployeeWithDept]">
                <item type="EmployeeWithDept">
                    <name type="str">Alice</name>
                    <id type="int">1</id>
                    <email type="str">alice@techcorp.com</email>
                    <department type="DepartmentWithId">
                        <id type="int">200</id>
                        <budget type="float">1000000.0</budget>
                    </department>
                </item>
            </team_members>
        </item>
    </projects>
</CompanyWithId>"""

parser_id = Parser(CompanyWithId)
parser_id.feed(xml_test_id)
result_id = parser_id.validate()

print(f"\nTesting with 'id' fields:")
print(f"  Company.id: {result_id.id} (expected: 999)")
print(f"  Company.name: {result_id.name} (expected: TechCorp)")
print(f"  Department id: {result_id.departments[0].id} (expected: 100)")
print(f"  Nested Department id: {result_id.projects[0].team_members[0].department.id} (expected: 200)")
print(f"  Is Company.id overwritten by nested dept id? {result_id.id == 200}")
