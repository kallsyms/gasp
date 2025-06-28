"""Test with the real example from the error"""

from typing import List, Union, Optional, Any
from gasp import Deserializable
from gasp.template_helpers import type_to_format_instructions

# Recreate the structure from the error
class MCPGetdiagnosticscodeTool(Deserializable):
    path: Optional[str]
    severities: Optional[List[Any]]
    format: Optional[str]
    includeSource: Optional[bool]

# Generate instructions
instructions = type_to_format_instructions(MCPGetdiagnosticscodeTool)

print("=== MCPGetdiagnosticscodeTool Instructions ===")
print(instructions)
print("\n=== End ===")

# Check what we get for list[Any]
print("\n=== Checking list[Any] handling ===")
if 'type="list[Any]"' in instructions:
    print("✓ Found list[Any] in type attribute")
else:
    print("✗ Did not find list[Any] in type attribute")

# Let's also test just List[Any] directly
list_any_instructions = type_to_format_instructions(List[Any])
print("\n=== Direct List[Any] Instructions ===")
print(list_any_instructions)
