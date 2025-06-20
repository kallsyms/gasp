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
    
    @classmethod
    def __gasp_from_partial__(cls, partial_data):
        print(f"Item.__gasp_from_partial__ called with keys: {list(partial_data.keys())}")
        instance = super().__gasp_from_partial__(partial_data)
        print(f"  Created Item: step={getattr(instance, 'step', 'NOT SET')}, title={getattr(instance, 'title', 'NOT SET')}")
        return instance
    
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
    
    @classmethod
    def __gasp_from_partial__(cls, partial_data):
        print(f"MetaPlan.__gasp_from_partial__ called with keys: {list(partial_data.keys())}")
        instance = super().__gasp_from_partial__(partial_data)
        print(f"After creation, instance has attributes: {list(instance.__dict__.keys())}")
        print(f"Items field: {getattr(instance, 'items', 'NOT SET')}")
        return instance
    
    def __eq__(self, other):
        if not isinstance(other, MetaPlan):
            return False
        return (self.title == other.title and
                self.summary == other.summary and
                self.items == other.items and
                self.reasoning == other.reasoning)


def test_incremental_list_coercion():
    """Test that nested objects in lists are coerced incrementally during streaming"""
    content = """<list type="list[Union[Chat, MetaPlan]]">
<item type="Chat">
  <content type="str">Hey! Alright, you want me to surprise you with a plan using all the tools? Let me cook up something interesting that showcases each one in a cohesive workflow!</content>
</item>
<item type="MetaPlan">
  <title type="str">Full-Stack Knowledge Management &amp; Development Workflow</title>
  <summary type="str">A comprehensive plan that demonstrates all available tools by creating a mini project management system with code, documentation, issue tracking, and knowledge capture.</summary>
  <items type="list[Item]">
    <item type="Item">
      <step type="int">1</step>
      <title type="str">Generate Core Application Code</title>
      <description type="str">Create a simple task management API with TypeScript/Node.js that demonstrates modern development patterns</description>
      <tool type="str">CodeTool</tool>
      <estimated_time type="str">5 minutes</estimated_time>
    </item>
    <item type="Item">
      <step type="int">2</step>
      <title type="str">Document Architecture Decisions</title>
      <description type="str">Save key technical decisions and patterns used in the codebase for future reference</description>
      <tool type="str">SaveKnowledge</tool>
      <estimated_time type="str">2 minutes</estimated_time>
    </item>
    <item type="Item">
      <step type="int">3</step>
      <title type="str">Create Development Issues</title>
      <description type="str">Generate GitHub issues for feature enhancements, bug fixes, and technical debt</description>
      <tool type="str">IssueForm</tool>
      <estimated_time type="str">3 minutes</estimated_time>
    </item>
    <item type="Item">
      <step type="int">4</step>
      <title type="str">Capture Project Insights</title>
      <description type="str">Save lessons learned and best practices discovered during the workflow</description>
      <tool type="str">SaveKnowledge</tool>
      <estimated_time type="str">2 minutes</estimated_time>
    </item>
  </items>
  <reasoning type="str">This plan creates a realistic development scenario that naturally uses every tool: MetaPlan for orchestration, CodeTool for implementation, SaveKnowledge for documentation and learning capture, IssueForm for project management, and Chat for context throughout. It's practical and demonstrates how these tools work together in real software development.</reasoning>
</item>
</list>"""
    
    parser = Parser(List[Union[Chat, MetaPlan]])
    
    # Feed content in small chunks to test incremental parsing
    chunk_size = 50
    chunks = [content[i:i+chunk_size] for i in range(0, len(content), chunk_size)]
    
    results = None
    for i, chunk in enumerate(chunks):
        results = parser.feed(chunk)
        if results:
            print(f"After chunk {i}, results: {results}")
            if len(results) > 1:
                print(f"Second item type: {type(results[1])}")
                if hasattr(results[1], '__dict__'):
                    print(f"Second item dict: {results[1].__dict__}")
    
    # Verify results
    assert results is not None
    assert len(results) == 2
    
    # Check first item (Chat)
    chat = results[0]
    assert isinstance(chat, Chat)
    assert "Hey! Alright" in chat.content
    
    # Check second item (MetaPlan)
    metaplan = results[1]
    print(f"MetaPlan type: {type(metaplan)}")
    print(f"MetaPlan attributes: {dir(metaplan)}")
    if hasattr(metaplan, '__dict__'):
        print(f"MetaPlan __dict__: {metaplan.__dict__}")
    assert isinstance(metaplan, MetaPlan)
    assert metaplan.title == "Full-Stack Knowledge Management & Development Workflow"
    assert hasattr(metaplan, 'items'), f"MetaPlan missing 'items' attribute. Has: {dir(metaplan)}"
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
    content = """<list type="list[Union[Chat, MetaPlan]]">
<item type="Chat">
  <content type="str">Simple chat message</content>
</item>
<item type="MetaPlan">
  <title type="str">Test Plan</title>
  <summary type="str">A test plan</summary>
  <items type="list[Item]">
    <item type="Item">
      <step type="int">1</step>
      <title type="str">First Step</title>
      <description type="str">Do something</description>
      <tool type="str">TestTool</tool>
      <estimated_time type="str">1 minute</estimated_time>
    </item>
  </items>
  <reasoning type="str">Test reasoning</reasoning>
</item>
</list>"""
    
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
  <name type="str">Test Container</name>
  <content type="Chat">
    <content type="str">Nested chat content</content>
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
  <name type="str">Empty</name>
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
  <title type="str">Step {i+1}</title>
  <description type="str">Description for step {i+1}</description>
  <tool type="str">Tool{i+1}</tool>
  <estimated_time type="str">{i+1} minutes</estimated_time>
</item>""")
    
    xml = f"""<list type="list[Item]">
{chr(10).join(items_xml)}
</list>"""
    
    # Stream in very small chunks to test incremental behavior
    chunk_size = 10
    chunks = [xml[i:i+chunk_size] for i in range(0, len(xml), chunk_size)]
    
    result = None
    items_with_data = 0
    first_populated_chunk = None
    
    for idx, chunk in enumerate(chunks):
        result = parser.feed(chunk)
        
        # Check item state during incremental parsing
        if result and len(result) > 0:
            # Count how many items have data
            current_items_with_data = sum(1 for item in result 
                                        if isinstance(item, Item) and 
                                        getattr(item, 'step', None) is not None)
            
            # Check if we just got new populated items
            if current_items_with_data > items_with_data:
                print(f"\nAfter chunk {idx} (fed '{chunk}'):")
                print(f"  Result has {len(result)} items, {current_items_with_data} with data")
                for i, item in enumerate(result):
                    if isinstance(item, Item):
                        print(f"  Item {i}: step={getattr(item, 'step', 'NOT SET')}, "
                              f"title={getattr(item, 'title', 'NOT SET')}, "
                              f"tool={getattr(item, 'tool', 'NOT SET')}")
                
                if first_populated_chunk is None:
                    first_populated_chunk = idx
                    print(f"  >>> First populated item appeared at chunk {idx}!")
                
                items_with_data = current_items_with_data
    
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
