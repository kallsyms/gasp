#!/usr/bin/env python3
"""Test gasp interpolate_prompt matching TypeParser pattern."""

from typing import List, Dict, Optional, Union, Type
from gasp import Deserializable, Parser
from gasp.template_helpers import interpolate_prompt

# Test classes using Deserializable for better incremental parsing
class Chat(Deserializable):
    message: str

class IssueForm(Deserializable):
    title: str
    description: str

class CodeTool(Deserializable):
    command: str
    language: str

class SaveKnowledge(Deserializable):
    content: str
    tags: List[str]

class MetaPlan(Deserializable):
    steps: List[str]

class WaitForConfirmation(Deserializable):
    prompt: str

class ActOnConfirmable(Deserializable):
    action: str

class Plan(Deserializable):
    tasks: List[str]

class PlanTask(Deserializable):
    task_id: str
    description: str

# Define the response type
ResponseType = List[Union[Chat, IssueForm, CodeTool, SaveKnowledge, MetaPlan, WaitForConfirmation, ActOnConfirmable, Plan, PlanTask]]

def test_interpolate_prompt():
    """Test interpolate_prompt matching TypeParser pattern."""
    print("=== Testing interpolate_prompt with TypeParser pattern ===\n")
    
    # Test prompt with {return_type} placeholder
    prompt = """You are an AI assistant.

Please respond with {return_type}.

Remember to format your response correctly."""
    
    # Mirror the TypeParser pattern: convert {return_type} to {{return_type}}
    type_placeholder_prompt = prompt.replace("{return_type}", "{{return_type}}")
    
    # Call interpolate_prompt with the type
    processed_prompt = interpolate_prompt(type_placeholder_prompt, ResponseType, format_tag="return_type")
    
    print("Original prompt:")
    print(prompt)
    print("\n" + "="*50 + "\n")
    print("Processed prompt with type instructions:")
    print(processed_prompt)
    print("\n" + "="*50 + "\n")
    
    # Test parsing with the generated format
    test_xml = """<List type="list[Chat | IssueForm | CodeTool | SaveKnowledge | MetaPlan | WaitForConfirmation | ActOnConfirmable | Plan | PlanTask]">
    <item type="Chat">
        <message type="str">Hello! I can help you with your software engineering questions.</message>
    </item>
</List>"""
    
    print("Test XML input:")
    print(test_xml)
    print("\n" + "="*50 + "\n")
    
    # Create parser and feed the XML
    parser = Parser(ResponseType)
    result = parser.feed(test_xml)
    
    print(f"Parsed result: {result}")
    print(f"Result type: {type(result)}")
    
    if isinstance(result, list) and len(result) > 0:
        first_item = result[0]
        print(f"First item: {first_item}")
        print(f"First item type: {type(first_item)}")
        print(f"Is Chat instance: {isinstance(first_item, Chat)}")
        if hasattr(first_item, 'message'):
            print(f"Message: {first_item.message}")

def test_simple_type_interpolation():
    """Test interpolate_prompt with simple types."""
    print("\n\n=== Testing simple type interpolation ===\n")
    
    # Test with a simple string type
    prompt = "Please provide {return_type}"
    type_placeholder_prompt = prompt.replace("{return_type}", "{{return_type}}")
    processed = interpolate_prompt(type_placeholder_prompt, str, format_tag="return_type")
    
    print("String type prompt:")
    print(processed)
    
    # Test with a list type
    prompt = "Give me {return_type}"
    type_placeholder_prompt = prompt.replace("{return_type}", "{{return_type}}")
    processed = interpolate_prompt(type_placeholder_prompt, List[int], format_tag="return_type")
    
    print("\n\nList[int] type prompt:")
    print(processed)

if __name__ == "__main__":
    test_interpolate_prompt()
    test_simple_type_interpolation()
