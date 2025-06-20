#!/usr/bin/env python3
"""Test Union types for tool-like classes"""

import pytest
from typing import Union, List
from gasp import Deserializable, Parser


class Search_webTool(Deserializable):
    """Tool for web searching"""
    query: str
    
    def __repr__(self):
        return f"Search_webTool(query={self.query!r})"
    
    def __eq__(self, other):
        if not isinstance(other, Search_webTool):
            return False
        return self.query == other.query


class Analyze_textTool(Deserializable):
    """Tool for text analysis"""
    text: str
    focus: str
    
    def __repr__(self):
        return f"Analyze_textTool(text={self.text!r}, focus={self.focus!r})"
    
    def __eq__(self, other):
        if not isinstance(other, Analyze_textTool):
            return False
        return self.text == other.text and self.focus == other.focus


class SearchResult(Deserializable):
    """Search result data"""
    title: str
    url: str
    snippet: str
    
    def __repr__(self):
        return f"SearchResult(title={self.title!r}, url={self.url!r}, snippet={self.snippet!r})"
    
    def __eq__(self, other):
        if not isinstance(other, SearchResult):
            return False
        return (self.title == other.title and 
                self.url == other.url and 
                self.snippet == other.snippet)


class ResearchReport(Deserializable):
    """Research report with findings"""
    topic: str
    summary: str
    key_findings: List[str]
    sources: List[SearchResult]
    
    def __repr__(self):
        return f"ResearchReport(topic={self.topic!r}, summary={self.summary!r}, findings={len(self.key_findings)}, sources={len(self.sources)})"
    
    def __eq__(self, other):
        if not isinstance(other, ResearchReport):
            return False
        return (self.topic == other.topic and 
                self.summary == other.summary and
                self.key_findings == other.key_findings and
                self.sources == other.sources)


# Create the union type
ToolUnion = Union[Search_webTool, Analyze_textTool, ResearchReport]


def test_search_web_tool_parsing():
    """Test parsing Search_webTool from Union"""
    parser = Parser(ToolUnion)
    
    # XML format with type attribute
    xml_data = '''<Search_webTool type="Search_webTool">
        <query type="str">latest advancements in quantum computing 2023</query>
    </Search_webTool>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, Search_webTool)
    assert result.query == "latest advancements in quantum computing 2023"


def test_analyze_text_tool_parsing():
    """Test parsing Analyze_textTool from Union"""
    parser = Parser(ToolUnion)
    
    xml_data = '''<Analyze_textTool type="Analyze_textTool">
        <text type="str">This is a sample text to analyze</text>
        <focus type="str">sentiment</focus>
    </Analyze_textTool>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, Analyze_textTool)
    assert result.text == "This is a sample text to analyze"
    assert result.focus == "sentiment"


def test_research_report_parsing():
    """Test parsing ResearchReport with nested structures"""
    parser = Parser(ToolUnion)
    
    xml_data = '''<ResearchReport type="ResearchReport">
        <topic type="str">Quantum Computing</topic>
        <summary type="str">Recent advances in quantum computing technology</summary>
        <key_findings type="list[str]">
            <item type="str">Breakthrough in error correction</item>
            <item type="str">New qubit design</item>
            <item type="str">Improved coherence times</item>
        </key_findings>
        <sources type="list[SearchResult]">
            <item type="SearchResult">
                <title type="str">Quantum Breakthrough 2023</title>
                <url type="str">https://example.com/quantum</url>
                <snippet type="str">Scientists achieve major breakthrough...</snippet>
            </item>
            <item type="SearchResult">
                <title type="str">New Qubit Architecture</title>
                <url type="str">https://example.com/qubits</url>
                <snippet type="str">Revolutionary qubit design shows promise...</snippet>
            </item>
        </sources>
    </ResearchReport>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, ResearchReport)
    assert result.topic == "Quantum Computing"
    assert result.summary == "Recent advances in quantum computing technology"
    assert len(result.key_findings) == 3
    assert result.key_findings[0] == "Breakthrough in error correction"
    assert len(result.sources) == 2
    assert isinstance(result.sources[0], SearchResult)
    assert result.sources[0].title == "Quantum Breakthrough 2023"


def test_tool_union_with_tag_discrimination():
    """Test Union discrimination based on tag names"""
    parser = Parser(ToolUnion)
    
    # Tag name should determine the type
    xml_data = '''<Search_webTool>
        <query type="str">machine learning trends</query>
    </Search_webTool>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, Search_webTool)
    assert result.query == "machine learning trends"


def test_tool_union_list():
    """Test list of tool union types"""
    parser = Parser(List[ToolUnion])
    
    xml_data = '''<list type="list[Union[Search_webTool, Analyze_textTool, ResearchReport]]">
        <item type="Search_webTool">
            <query type="str">AI research papers</query>
        </item>
        <item type="Analyze_textTool">
            <text type="str">Sample analysis text</text>
            <focus type="str">keywords</focus>
        </item>
        <item type="Search_webTool">
            <query type="str">neural networks</query>
        </item>
    </list>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert len(result) == 3
    
    assert isinstance(result[0], Search_webTool)
    assert result[0].query == "AI research papers"
    
    assert isinstance(result[1], Analyze_textTool)
    assert result[1].text == "Sample analysis text"
    assert result[1].focus == "keywords"
    
    assert isinstance(result[2], Search_webTool)
    assert result[2].query == "neural networks"


def test_tool_union_streaming():
    """Test streaming parsing of tool unions"""
    parser = Parser(ToolUnion)
    
    chunks = [
        '<Analyze_textTool type="Analyze_textTool">',
        '<text type="str">Streaming ',
        'text content</text>',
        '<focus type="str">summary</focus>',
        '</Analyze_textTool>'
    ]
    
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    validated = parser.validate()
    assert validated is not None
    assert isinstance(validated, Analyze_textTool)
    assert validated.text == "Streaming text content"
    assert validated.focus == "summary"


def test_nested_tool_container():
    """Test tools nested in a container"""
    class ToolContainer(Deserializable):
        name: str
        tool: ToolUnion
        timestamp: str
    
    parser = Parser(ToolContainer)
    
    xml_data = '''<ToolContainer>
        <name type="str">Research Task</name>
        <tool type="Search_webTool">
            <query type="str">latest AI developments</query>
        </tool>
        <timestamp type="str">2023-12-01T10:00:00</timestamp>
    </ToolContainer>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, ToolContainer)
    assert result.name == "Research Task"
    assert isinstance(result.tool, Search_webTool)
    assert result.tool.query == "latest AI developments"
    assert result.timestamp == "2023-12-01T10:00:00"


def test_empty_sources_in_report():
    """Test ResearchReport with empty sources list"""
    parser = Parser(ResearchReport)
    
    xml_data = '''<ResearchReport>
        <topic type="str">Empty Report</topic>
        <summary type="str">No sources</summary>
        <key_findings type="list[str]">
        </key_findings>
        <sources type="list[SearchResult]">
        </sources>
    </ResearchReport>'''
    
    parser.feed(xml_data)
    result = parser.validate()
    
    assert result is not None
    assert isinstance(result, ResearchReport)
    assert result.topic == "Empty Report"
    assert result.key_findings == []
    assert result.sources == []


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
