"""Test template generation for List[Union[...]] types"""

import pytest
from typing import List, Union, Optional
from gasp import Parser, Deserializable
from gasp.template_helpers import type_to_format_instructions


class IssueForm(Deserializable):
    """A form for creating GitHub issues"""
    title: str
    body: str


class WaitForConfirmation(Deserializable):
    """A confirmation prompt"""
    prompt: str


class Chat(Deserializable):
    """A chat message"""
    content: str


# Define the union type
ActionType = Union[Chat, IssueForm, WaitForConfirmation]


def test_union_list_format():
    """Test that List[Union[...]] generates proper structure examples"""
    
    # Test the List[Union[...]] format
    instructions = type_to_format_instructions(List[ActionType])
    
    # Check that structure examples are included
    assert "When you see 'IssueForm' in a type attribute" in instructions
    assert "When you see 'WaitForConfirmation' in a type attribute" in instructions
    assert "When you see 'Chat' in a type attribute" in instructions
    
    # Check that the IssueForm structure uses correct format
    assert '<title type="str">' in instructions
    assert '<body type="str">' in instructions
    
    # Make sure there are no <key> tags
    assert "<key" not in instructions
    
    # Verify the List type attribute shows the union
    assert 'type="list[Chat | IssueForm | WaitForConfirmation]"' in instructions


def test_dict_format():
    """Test that dict format uses <item key='...'> not <key>"""
    
    # Test a simple dict
    dict_instructions = type_to_format_instructions(dict[str, str])
    
    # Verify dict uses <item key="...">
    assert '<item key=' in dict_instructions
    assert '<key' not in dict_instructions
    
    # Check the format is correct
    assert '<item key="example_key1" type="str">example string</item>' in dict_instructions


def test_union_list_no_structure_for_primitives():
    """Test that List[Union[str, int]] doesn't generate structure examples"""
    
    # Test List[Union[str, int]]
    instructions = type_to_format_instructions(List[Union[str, int]])
    
    # Should not have structure examples for primitives
    assert "When you see 'str' in a type attribute" not in instructions
    assert "When you see 'int' in a type attribute" not in instructions
    
    # Should show the union type
    assert 'type="list[str | int]"' in instructions


def test_nested_dict_in_list():
    """Test List[dict[str, IssueForm]] generates correct structure"""
    
    instructions = type_to_format_instructions(List[dict[str, IssueForm]])
    
    # Should have structure example for IssueForm
    assert "When you see 'IssueForm' in a type attribute" in instructions
    
    # Dict items should use correct format
    assert '<item key=' in instructions
    assert '<key' not in instructions


# Define type alias using type statement
type AgentAction = Chat | IssueForm | WaitForConfirmation


# Torture test classes for deeply nested structures
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


def test_deeply_nested_torture_test():
    """Test extremely deeply nested structures with mixed optional/non-optional containers"""
    
    instructions = type_to_format_instructions(Company)
    
    print("\n=== Deeply Nested Torture Test ===")
    print(instructions)
    print("=== End ===\n")
    
    # Basic structure checks - look for the type names mentioned
    assert "Company" in instructions
    assert "name" in instructions
    assert "str" in instructions
    
    # Check that container types are mentioned
    assert "list" in instructions
    assert "dict" in instructions 
    assert "tuple" in instructions
    assert "set" in instructions
    assert "Optional" in instructions
    
    # Check field names are mentioned
    assert "departments" in instructions
    assert "projects" in instructions
    assert "employee_directory" in instructions
    assert "regional_data" in instructions
    assert "company_tags" in instructions
    assert "announcements" in instructions
    assert "quarterly_reports" in instructions
    assert "metadata" in instructions
    assert "initiatives" in instructions
    
    # Verify structure examples are included for custom classes
    assert "When you see 'Department' in a type attribute" in instructions
    assert "When you see 'Employee' in a type attribute" in instructions
    assert "When you see 'Project' in a type attribute" in instructions
    
    # Check that nested class field names are mentioned
    assert "budget" in instructions
    assert "email" in instructions
    assert "team_members" in instructions
    assert "milestones" in instructions
    
    # Check that union types are handled
    assert "Project | Department" in instructions or ("Project" in instructions and "Department" in instructions)


def test_simple_company_parsing():
    """Test parsing a simple company structure first"""
    
    # Start with a simpler Company class
    class SimpleCompany(Deserializable):
        name: str
        departments: List[Department]
        employee_directory: dict[str, Employee]
    
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
    
    parser = Parser(SimpleCompany)
    
    # Enable debug to see what's happening
    import os
    os.environ['GASP_DEBUG'] = '1'
    
    parser.feed(xml)
    
    # Check partial state before validation
    partial = parser.get_partial()
    print(f"\nPartial object before validation: {partial}")
    print(f"Is complete: {parser.is_complete()}")
    if partial:
        print(f"Name: {getattr(partial, 'name', 'NOT SET')}")
        print(f"Departments: {getattr(partial, 'departments', 'NOT SET')}")
        print(f"Employee directory: {getattr(partial, 'employee_directory', 'NOT SET')}")
    
    result = parser.validate()
    
    assert result is not None
    assert result.name == "TechCorp"
    assert len(result.departments) == 1
    assert result.departments[0].name == "Engineering"
    
    # Check the dict parsing
    print(f"Employee directory: {result.employee_directory}")
    assert len(result.employee_directory) > 0, "Employee directory should not be empty"
    assert "alice" in result.employee_directory
    assert result.employee_directory["alice"].name == "Alice"


def test_torture_test_with_actual_parsing():
    """Test that we can actually parse a deeply nested structure"""
    
    # For now, let's use a simpler XML that focuses on the core issue
    xml = """<Company>
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
    
    # This is the torture test - can the parser handle all this nesting?
    parser = Parser(Company)
    parser.feed(xml)
    result = parser.validate()
    
    # Make sure we got a result
    assert result is not None
    
    # Basic assertions
    assert result.name == "TechCorp"
    assert len(result.departments) == 2
    assert result.departments[0].name == "Engineering"
    assert result.departments[0].budget == 1000000.0
    
    # Check optional list worked
    assert result.projects is not None
    assert len(result.projects) == 1
    assert result.projects[0].title == "AI Platform"
    assert len(result.projects[0].team_members) == 1
    assert result.projects[0].team_members[0].name == "Alice"
    
    # Check dict
    assert "alice" in result.employee_directory
    assert result.employee_directory["alice"].email == "alice@techcorp.com"
    
    # Check deeply nested optional dict of lists of tuples
    assert result.regional_data is not None
    assert "North America" in result.regional_data
    na_data = result.regional_data["North America"]
    assert len(na_data) == 2
    assert na_data[0] == ("USA", 100, 95.5)
    assert na_data[1] == ("Canada", 50, None)
    
    # Check optional set
    assert result.company_tags is not None
    assert result.company_tags == {"tech", "startup", "AI"}
    
    # Check union list
    assert len(result.announcements) == 2
    assert result.announcements[0] == "Company picnic next Friday!"
    assert isinstance(result.announcements[1], dict)
    assert result.announcements[1]["type"] == "funding"
    assert result.announcements[1]["amount"] == 10000000
    
    # Check nested optional lists
    assert result.quarterly_reports is not None
    assert len(result.quarterly_reports) == 2
    assert result.quarterly_reports[0] is not None
    assert result.quarterly_reports[0]["revenue"] == 1500000.0
    assert result.quarterly_reports[0]["highlights"] == ["New product launch", "Exceeded targets"]
    assert result.quarterly_reports[1] is None
    
    # Check tuple with mixed types
    assert result.metadata[0] == "TechCorp Metadata"
    assert result.metadata[1] == 2024
    assert result.metadata[2] is not None
    assert len(result.metadata[2]) == 1
    assert "ceo" in result.metadata[2][0]
    assert result.metadata[2][0]["ceo"].name == "Bob"
    
    # Check super nested initiatives
    assert len(result.initiatives) == 2
    assert "Q1-initiatives" in result.initiatives[0]
    q1_list = result.initiatives[0]["Q1-initiatives"]
    assert q1_list is not None
    assert len(q1_list) == 2
    assert isinstance(q1_list[0], Project)
    assert q1_list[0].title == "Mobile App"
    assert isinstance(q1_list[1], Department)
    assert q1_list[1].name == "Mobile Division"
    assert result.initiatives[1]["Q2-initiatives"] is None


def test_type_alias_with_type_statement():
    """Test that 'type X = Union[...]' syntax generates correct format"""
    
    # Debug: Let's see what the type looks like
    from typing import get_origin, get_args
    print(f"\n=== Debug type alias ===")
    print(f"AgentAction: {AgentAction}")
    print(f"type(AgentAction): {type(AgentAction)}")
    print(f"hasattr __value__: {hasattr(AgentAction, '__value__')}")
    if hasattr(AgentAction, '__value__'):
        print(f"AgentAction.__value__: {AgentAction.__value__}")
    print(f"List[AgentAction]: {List[AgentAction]}")
    list_args = get_args(List[AgentAction])
    print(f"get_args(List[AgentAction]): {list_args}")
    if list_args:
        print(f"list_args[0]: {list_args[0]}")
        print(f"type(list_args[0]): {type(list_args[0])}")
        print(f"hasattr __value__: {hasattr(list_args[0], '__value__')}")
    
    # Test the List[AgentAction] format
    instructions = type_to_format_instructions(List[AgentAction])
    
    print(f"\n=== Generated instructions ===")
    print(instructions)
    print("=== End instructions ===\n")
    
    # The type alias should be resolved to show the union members
    assert "When you see 'IssueForm' in a type attribute" in instructions
    assert "When you see 'WaitForConfirmation' in a type attribute" in instructions
    assert "When you see 'Chat' in a type attribute" in instructions
    
    # Check the list type attribute - it should show the expanded union
    assert 'type="list[Chat | IssueForm | WaitForConfirmation]"' in instructions
