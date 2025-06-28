"""Test the fix for nested union structure examples"""

from typing import List, Union
from gasp import Deserializable
from gasp.template_helpers import type_to_format_instructions

# Define the component classes
class Chat(Deserializable):
    content: str

class IssueForm(Deserializable):
    title: str
    body: str

class CodeTool(Deserializable):
    language: str
    code: str

class SaveKnowledge(Deserializable):
    topic: str
    content: str

class MCPListfilescodeTool(Deserializable):
    path: str
    recursive: bool

class MCPReadfilecodeTool(Deserializable):
    path: str

# Create the nested union type alias
type AgentAction = Union[Chat, IssueForm, CodeTool, SaveKnowledge]

# Create the complex list type
ComplexList = List[Union[AgentAction, MCPListfilescodeTool, MCPReadfilecodeTool]]

# Generate instructions
instructions = type_to_format_instructions(ComplexList)

print("=== Generated Instructions ===")
print(instructions)
print("\n=== Checking for structure examples ===")

# Check that all types have structure examples
expected_types = ['Chat', 'IssueForm', 'CodeTool', 'SaveKnowledge', 
                  'MCPListfilescodeTool', 'MCPReadfilecodeTool']

for type_name in expected_types:
    if f"When you see '{type_name}' in a type attribute" in instructions:
        print(f"✓ Found structure example for {type_name}")
    else:
        print(f"✗ Missing structure example for {type_name}")

# Check the type string
if 'type="list[Chat | IssueForm | CodeTool | SaveKnowledge | MCPListfilescodeTool | MCPReadfilecodeTool]"' in instructions:
    print("\n✓ Type string is correctly flattened")
else:
    print("\n✗ Type string is not correctly flattened")
