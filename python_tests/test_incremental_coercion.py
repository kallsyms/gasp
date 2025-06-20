#!/usr/bin/env python3
"""
Test incremental coercion of nested objects during streaming
"""

import pytest
from gasp import Deserializable, Parser
from typing import List, Union


class Chat(Deserializable):
    content: str
    
    def __eq__(self, other):
        if not isinstance(other, Chat):
            return False
        return self.content == other.content


class Item(Deserializable):
    step: int
    title: str
    description: str
    tool: str
    estimated_time: str
    
    def __eq__(self, other):
        if not isinstance(other, Item):
            return False
        return (self.step == other.step and 
                self.title == other.title and
                self.description == other.description and
                self.tool == other.tool and
                self.estimated_time == other.estimated_time)


class MetaPlan(Deserializable):
    title: str
    summary: str
    items: List[Item]
    reasoning: str
    
    def __eq__(self, other):
        if not isinstance(other, MetaPlan):
            return False
        return (self.title == other.title and
                self.summary == other.summary and
                self.items == other.items and
                self.reasoning == other.reasoning)


def test_incremental_list_coercion():
    """Test that nested objects in lists are coerced incrementally during streaming"""
    content = """<List type="list[Union[Chat, MetaPlan]]">
<item type="Chat">
  <content type="string">Hey! Alright, you want me to surprise you with a plan using all the tools? Let me cook up something interesting that showcases each one in a cohesive workflow!</content>
</item>
<item type="MetaPlan">
  <title type="string">Full-Stack Knowledge Management &amp; Development Workflow</title>
  <summary type="string">A comprehensive plan that demonstrates all available tools by creating a mini project management system with code, documentation, issue tracking, and knowledge capture.</summary>
  <items type="list[Item]">
    <item type="Item">
      <step type="int">1</step>
      <title type="string">Generate Core Application Code</title>
      <description type="string">Create a simple task management API with TypeScript/Node.js that demonstrates modern development patterns</description>
      <tool type="string">CodeTool</tool>
      <estimated_time type="string">5 minutes</estimated_time>
    </item>
    <item type="Item">
      <step type="int">2</step>
      <title type="string">Document Architecture Decisions</title>
      <description type="string">Save key technical decisions and patterns used in the codebase for future reference</description>
      <tool type="string">SaveKnowledge</tool>
      <estimated_time type="string">2 minutes</estimated_time>
    </item>
    <item type="Item">
      <step type="int">3</step>
      <title type="string">Create Development Issues</title>
      <description type="string">Generate GitHub issues for feature enhancements, bug fixes, and technical debt</description>
      <tool type="string">IssueForm</tool>
      <estimated_time type="string">3 minutes</estimated_time>
    </item>
    <item type="Item">
      <step type="int">4</step>
      <title type="string">Capture Project Insights</title>
      <description type="string">Save lessons learned and best practices discovered during the workflow</description>
      <tool type="string">SaveKnowledge</tool>
      <estimated_time type="string">2 minutes</estimated_time>
    </item>
  </items>
  <reasoning type="string">This plan creates a realistic development scenario that naturally uses every tool: MetaPlan for orchestration, CodeTool for implementation, SaveKnowledge for documentation and learning capture, IssueForm for project management, and Chat for context throughout. It's practical and demonstrates how these tools work together in real software development.</reasoning>
</item>
</List>"""
    
    parser = Parser(List[Union[Chat, MetaPlan]])
    
    # Feed content in small chunks to test incremental parsing
    chunk_size = 50
    chunks = [content[i:i+chunk_size] for i in range(0, len(content), chunk_size)]
    
    results = None
    for chunk in chunks:
        results = parser.feed(chunk)
    
    # Verify results
    assert results is not None
    assert len(results) == 2
    
    # Check first item (Chat)
    chat = results[0]
    assert isinstance(chat, Chat)
    assert "Hey! Alright" in chat.content
    
    # Check second item (MetaPlan)
    metaplan = results[1]
    assert isinstance(metaplan, MetaPlan)
    assert metaplan.title == "Full-Stack Knowledge Management & Development Workflow"
    assert len(metaplan.items) == 4
    
    # Verify all items are properly coerced to Item instances
    assert all(isinstance(item, Item) for item in metaplan.items)
    
    # Check specific items
    assert metaplan.items[0].step == 1
    assert metaplan.items[0].title == "Generate Core Application Code"
    assert metaplan.items[3].step == 4
    assert metaplan.items[3].title == "Capture Project Insights"


def test_incremental_coercion_tracking():
    """Test that we can track when objects are coerced during streaming"""
    parser = Parser(List[Union[Chat, MetaPlan]])
    
    # Simple content to test coercion
    content = """<List type="list[Union[Chat, MetaPlan]]">
<item type="Chat">
  <content type="string">Simple chat message</content>
</item>
<item type="MetaPlan">
  <title type="string">Test Plan</title>
  <summary type="string">A test plan</summary>
  <items type="list[Item]">
    <item type="Item">
      <step type="int">1</step>
      <title type="string">First Step</title>
      <description type="string">Do something</description>
      <tool type="string">TestTool</tool>
      <estimated_time type="string">1 minute</estimated_time>
    </item>
  </items>
  <reasoning type="string">Test reasoning</reasoning>
</item>
</List>"""
    
    # Track when items are coerced
    item_coerced = False
    coercion_chunk = -1
    results = None
    
    chunk_size = 20
    chunks = [content[i:i+chunk_size] for i in range(0, len(content), chunk_size)]
    
    for i, chunk in enumerate(chunks):
        results = parser.feed(chunk)
        
        # Check if items have been coerced
        if not item_coerced and results and len(results) > 1:
            metaplan = results[1]
            if hasattr(metaplan, 'items') and metaplan.items:
                for item in metaplan.items:
                    if isinstance(item, Item):
                        item_coerced = True
                        coercion_chunk = i
                        break
    
    assert item_coerced
    assert coercion_chunk >= 0
    assert results is not None
    assert len(results) == 2


def test_nested_union_streaming():
    """Test streaming with nested unions"""
    class Container(Deserializable):
        name: str
        content: Union[Chat, MetaPlan]
    
    parser = Parser(Container)
    
    xml = """<Container>
  <name type="string">Test Container</name>
  <content type="Chat">
    <content type="string">Nested chat content</content>
  </content>
</Container>"""
    
    # Stream in small chunks
    chunk_size = 30
    chunks = [xml[i:i+chunk_size] for i in range(0, len(xml), chunk_size)]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert isinstance(result, Container)
    assert result.name == "Test Container"
    assert isinstance(result.content, Chat)
    assert result.content.content == "Nested chat content"


def test_empty_list_incremental():
    """Test incremental parsing with empty lists"""
    class EmptyListContainer(Deserializable):
        name: str
        items: List[Item]
    
    parser = Parser(EmptyListContainer)
    
    xml = """<EmptyListContainer>
  <name type="string">Empty</name>
  <items type="list[Item]">
  </items>
</EmptyListContainer>"""
    
    # Stream in chunks
    chunk_size = 25
    chunks = [xml[i:i+chunk_size] for i in range(0, len(xml), chunk_size)]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert result.name == "Empty"
    assert result.items == []


def test_large_list_incremental():
    """Test incremental parsing with larger lists"""
    parser = Parser(List[Item])
    
    # Generate XML for multiple items
    items_xml = []
    for i in range(5):
        items_xml.append(f"""<item type="Item">
  <step type="int">{i+1}</step>
  <title type="string">Step {i+1}</title>
  <description type="string">Description for step {i+1}</description>
  <tool type="string">Tool{i+1}</tool>
  <estimated_time type="string">{i+1} minutes</estimated_time>
</item>""")
    
    xml = f"""<List type="list[Item]">
{chr(10).join(items_xml)}
</List>"""
    
    # Stream in very small chunks to test incremental behavior
    chunk_size = 10
    chunks = [xml[i:i+chunk_size] for i in range(0, len(xml), chunk_size)]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert len(result) == 5
    assert all(isinstance(item, Item) for item in result)
    
    # Verify items
    for i, item in enumerate(result):
        assert item.step == i + 1
        assert item.title == f"Step {i+1}"
        assert item.estimated_time == f"{i+1} minutes"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
