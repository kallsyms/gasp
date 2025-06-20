#!/usr/bin/env python3
"""
GASP Nested Type Conversion Test

This script tests that nested types in collections are properly converted
to their correct Python types, rather than remaining as dictionaries.
"""

import pytest
from gasp import Deserializable, Parser
from typing import List, Optional


class Subsystem(Deserializable):
    """A subsystem with name and category"""
    name: str
    category: str
    priority: int = 0  # Default value
    
    def __repr__(self):
        return f"Subsystem(name='{self.name}', category='{self.category}', priority={self.priority})"
    
    def __eq__(self, other):
        if not isinstance(other, Subsystem):
            return False
        return self.name == other.name and self.category == other.category and self.priority == other.priority


class ReportSubsystems(Deserializable):
    """Report of subsystems found in the codebase"""
    subsystems: List[Subsystem]
    
    def __init__(self, subsystems: List[Subsystem] | None = None):
        self.subsystems = subsystems or []
    
    def __repr__(self):
        return f"ReportSubsystems(subsystems={self.subsystems})"
    
    def __eq__(self, other):
        if not isinstance(other, ReportSubsystems):
            return False
        return self.subsystems == other.subsystems


def test_nested_type_conversion():
    """Test that nested types in collections are properly converted"""
    # Create a parser for the ReportSubsystems type
    parser = Parser(ReportSubsystems)
    
    # Test with XML that includes nested Subsystem objects
    xml_data = '''<ReportSubsystems>
        <subsystems type="list[Subsystem]">
            <item type="Subsystem">
                <name type="str">Authentication</name>
                <category type="str">Security</category>
                <priority type="int">1</priority>
            </item>
            <item type="Subsystem">
                <name type="str">Database</name>
                <category type="str">Storage</category>
                <priority type="int">2</priority>
            </item>
        </subsystems>
    </ReportSubsystems>'''
    
    # Feed the data to the parser
    result = parser.feed(xml_data)
    
    # Check if the parser is complete
    assert parser.is_complete()
    
    # Validate the result
    validated = parser.validate()
    assert validated is not None
    
    # Check the types to verify nested objects are properly instantiated
    assert isinstance(validated, ReportSubsystems)
    assert hasattr(validated, 'subsystems')
    assert isinstance(validated.subsystems, list)
    assert len(validated.subsystems) == 2
    
    # Check first subsystem
    subsystem1 = validated.subsystems[0]
    assert isinstance(subsystem1, Subsystem)
    assert subsystem1.name == "Authentication"
    assert subsystem1.category == "Security"
    assert subsystem1.priority == 1
    
    # Check second subsystem
    subsystem2 = validated.subsystems[1]
    assert isinstance(subsystem2, Subsystem)
    assert subsystem2.name == "Database"
    assert subsystem2.category == "Storage"
    assert subsystem2.priority == 2


def test_nested_with_default_values():
    """Test nested types with default values"""
    parser = Parser(ReportSubsystems)
    
    # Test with XML that omits the priority field (should use default)
    xml_data = '''<ReportSubsystems>
        <subsystems type="list[Subsystem]">
            <item type="Subsystem">
                <name type="str">Logging</name>
                <category type="str">Infrastructure</category>
            </item>
        </subsystems>
    </ReportSubsystems>'''
    
    result = parser.feed(xml_data)
    validated = parser.validate()
    
    assert validated is not None
    assert len(validated.subsystems) == 1
    
    subsystem = validated.subsystems[0]
    assert isinstance(subsystem, Subsystem)
    assert subsystem.name == "Logging"
    assert subsystem.category == "Infrastructure"
    assert subsystem.priority == 0  # Default value


def test_empty_nested_list():
    """Test with empty list of nested objects"""
    parser = Parser(ReportSubsystems)
    
    xml_data = '''<ReportSubsystems>
        <subsystems type="list[Subsystem]">
        </subsystems>
    </ReportSubsystems>'''
    
    result = parser.feed(xml_data)
    validated = parser.validate()
    
    assert validated is not None
    assert isinstance(validated, ReportSubsystems)
    assert validated.subsystems == []


def test_streaming_nested_types():
    """Test streaming parsing of nested types"""
    parser = Parser(ReportSubsystems)
    
    chunks = [
        '<ReportSubsystems>',
        '<subsystems type="list[Subsystem]">',
        '<item type="Subsystem">',
        '<name type="str">Cache</name>',
        '<category type="str">Performance</category>',
        '<priority type="int">3</priority>',
        '</item>',
        '</subsystems>',
        '</ReportSubsystems>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert parser.is_complete()
    validated = parser.validate()
    
    assert validated is not None
    assert len(validated.subsystems) == 1
    assert validated.subsystems[0].name == "Cache"
    assert validated.subsystems[0].category == "Performance"
    assert validated.subsystems[0].priority == 3


class NestedContainer(Deserializable):
    """Container with nested lists of objects"""
    items: List[List[Subsystem]]
    
    def __eq__(self, other):
        if not isinstance(other, NestedContainer):
            return False
        return self.items == other.items


def test_deeply_nested_types():
    """Test deeply nested type structures"""
    parser = Parser(NestedContainer)
    
    xml_data = '''<NestedContainer>
        <items type="list[list[Subsystem]]">
            <item type="list[Subsystem]">
                <item type="Subsystem">
                    <name type="str">API</name>
                    <category type="str">Interface</category>
                    <priority type="int">1</priority>
                </item>
                <item type="Subsystem">
                    <name type="str">GraphQL</name>
                    <category type="str">Interface</category>
                    <priority type="int">2</priority>
                </item>
            </item>
            <item type="list[Subsystem]">
                <item type="Subsystem">
                    <name type="str">Redis</name>
                    <category type="str">Cache</category>
                    <priority type="int">1</priority>
                </item>
            </item>
        </items>
    </NestedContainer>'''
    
    result = parser.feed(xml_data)
    validated = parser.validate()
    
    assert validated is not None
    assert isinstance(validated, NestedContainer)
    assert len(validated.items) == 2
    
    # Check first inner list
    assert len(validated.items[0]) == 2
    assert all(isinstance(item, Subsystem) for item in validated.items[0])
    assert validated.items[0][0].name == "API"
    assert validated.items[0][1].name == "GraphQL"
    
    # Check second inner list
    assert len(validated.items[1]) == 1
    assert isinstance(validated.items[1][0], Subsystem)
    assert validated.items[1][0].name == "Redis"


class OptionalNested(Deserializable):
    """Class with optional nested object"""
    name: str
    subsystem: Optional[Subsystem] = None
    
    def __eq__(self, other):
        if not isinstance(other, OptionalNested):
            return False
        return self.name == other.name and self.subsystem == other.subsystem


def test_optional_nested_type():
    """Test optional nested type handling"""
    parser = Parser(OptionalNested)
    
    # Test with nested object present
    xml_data1 = '''<OptionalNested>
        <name type="str">System A</name>
        <subsystem type="Subsystem">
            <name type="str">Core</name>
            <category type="str">Main</category>
            <priority type="int">1</priority>
        </subsystem>
    </OptionalNested>'''
    
    result1 = parser.feed(xml_data1)
    validated1 = parser.validate()
    
    assert validated1 is not None
    assert validated1.name == "System A"
    assert validated1.subsystem is not None
    assert isinstance(validated1.subsystem, Subsystem)
    assert validated1.subsystem.name == "Core"
    
    # Test with nested object absent
    parser2 = Parser(OptionalNested)
    xml_data2 = '''<OptionalNested>
        <name type="str">System B</name>
    </OptionalNested>'''
    
    result2 = parser2.feed(xml_data2)
    validated2 = parser2.validate()
    
    assert validated2 is not None
    assert validated2.name == "System B"
    assert validated2.subsystem is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
