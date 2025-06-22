"""Debug the list union parsing issue"""

from typing import List, Union, Optional
from gasp import Parser, Deserializable


class IssueForm(Deserializable):
  provider: str
  title: str
  description: str
  labels: List[str]
  assignees: List[str]
  bismuth_assigned: Optional[bool]
  status: str
  priority: str

class Chat(Deserializable):
  content: str

class SaveKnowledge(Deserializable):
  title: str
  content: str
  category: Optional[str]
  tags: Optional[List[str]]
  importance: Optional[str]  # "low" | "medium" | "high"
  context: str
  source: Optional[str]

class WaitForConfirmation(Deserializable):
  prompt: str

# Define type alias
type ActionList = Union[Chat, SaveKnowledge, IssueForm, WaitForConfirmation]

xml = '''
<List type="list[Chat | SaveKnowledge | IssueForm | WaitForConfirmation]">
    <item type="Chat">
        <content type="str">You got it. I'll create that Linear ticket for the frontend bug. Here are the details I've put together:</content>
    </item>
    <item type="SaveKnowledge">
        <title type="str">Frontend Post Bug: Profile Data Missing</title>
        <content type="str">A ticket needs to be created for a frontend issue where profile data fails to display after making a post.</content>
        <category type="str">Known Issues</category>
        <tags type="list[str]">
            <item type="str">bug</item>
            <item type="str">frontend</item>
        </tags>
        <importance type="str">Medium</importance>
        <context type="str">The user asked to create a ticket for
'''

# Try to parse
parser = Parser(List[ActionList])

# Add debug to trace the issue
print("Parsing XML with nested list in SaveKnowledge.tags field...")
print("Expected: List with 2 items - Chat and SaveKnowledge (with tags=['bug', 'frontend'])")
print()

try:
    result = parser.feed(xml)
    print(f"Result: {result}")
    if result:
        print(f"Result length: {len(result)}")
        for i, item in enumerate(result):
            print(f"  [{i}] {type(item).__name__}: {item}")
            if hasattr(item, '__dict__'):
                for k, v in item.__dict__.items():
                    print(f"      {k}: {v}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()
