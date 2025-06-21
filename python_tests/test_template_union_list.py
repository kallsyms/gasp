"""Test template generation for List[Union[...]] types"""

import pytest
from typing import List, Union
from gasp.template_helpers import type_to_format_instructions


class IssueForm:
    """A form for creating GitHub issues"""
    title: str
    body: str


class WaitForConfirmation:
    """A confirmation prompt"""
    prompt: str


class Chat:
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
