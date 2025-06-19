import pytest
from gasp import Deserializable
from gasp.gasp import Parser
from typing import List, Union

class Chat(Deserializable):
    content: str

class Item(Deserializable):
    step: int
    title: str
    description: str
    tool: str
    estimated_time: str

class MetaPlan(Deserializable):
    title: str
    summary: str
    items: List[Item]
    reasoning: str

@pytest.mark.asyncio
async def test_incremental_list_coercion():
    content = """<List>[
{
  "_type_name": "Chat",
  "content": "Hey! Alright, you want me to surprise you with a plan using all the tools? Let me cook up something interesting that showcases each one in a cohesive workflow!"
},
{
  "_type_name": "MetaPlan",
  "title": "Full-Stack Knowledge Management & Development Workflow",
  "summary": "A comprehensive plan that demonstrates all available tools by creating a mini project management system with code, documentation, issue tracking, and knowledge capture.",
  "items": [
    {
      "step": 1,
      "title": "Generate Core Application Code",
      "description": "Create a simple task management API with TypeScript/Node.js that demonstrates modern development patterns",
      "tool": "CodeTool",
      "estimated_time": "5 minutes"
    },
    {
      "step": 2,
      "title": "Document Architecture Decisions",
      "description": "Save key technical decisions and patterns used in the codebase for future reference",
      "tool": "SaveKnowledge",
      "estimated_time": "2 minutes"
    },
    {
      "step": 3,
      "title": "Create Development Issues",
      "description": "Generate GitHub issues for feature enhancements, bug fixes, and technical debt",
      "tool": "IssueForm",
      "estimated_time": "3 minutes"
    },
    {
      "step": 4,
      "title": "Capture Project Insights",
      "description": "Save lessons learned and best practices discovered during the workflow",
      "tool": "SaveKnowledge",
      "estimated_time": "2 minutes"
    }
  ],
    "reasoning": "This plan creates a realistic development scenario that naturally uses every tool: MetaPlan for orchestration, CodeTool for implementation, SaveKnowledge for documentation and learning capture, IssueForm for project management, and Chat for context throughout. It's practical and demonstrates how these tools work together in real software development."
}]
</List>
"""
    parser = Parser(List[Union[Chat, MetaPlan]])
    results = None
    item_coerced = False
    total_chunks = len(content) // 5
    for i, chunk in enumerate([content[i:i+5] for i in range(0, len(content), 5)]):
        results = parser.feed(chunk)
        if not item_coerced and results and len(results) > 1 and results[1] is not None:
            if hasattr(results[1], 'items'):
                for item in results[1].items:
                    if item is not None:
                        print(f"First item coerced at chunk {i} of {total_chunks}")
                        item_coerced = True
                        break

    assert item_coerced
    assert results is not None
    assert len(results) == 2
    chat = results[0]
    assert isinstance(chat, Chat)
    metaplan = results[1]
    assert isinstance(metaplan, MetaPlan)
    assert len(metaplan.items) == 4
    assert all(isinstance(item, Item) for item in metaplan.items)
    assert metaplan.items[0].step == 1
    assert metaplan.items[3].title == "Capture Project Insights"
