#!/usr/bin/env python3
"""
Test demonstrating tag filtering with gasp.

This shows how to use ignored tags to filter out unwanted content.
"""

import pytest
import gasp
from typing import List, Dict


class Result(gasp.Deserializable):
    answer: str
    confidence: float
    
    def __eq__(self, other):
        if not isinstance(other, Result):
            return False
        return self.answer == other.answer and self.confidence == other.confidence


class Response(gasp.Deserializable):
    data: str
    status: str
    
    def __eq__(self, other):
        if not isinstance(other, Response):
            return False
        return self.data == other.data and self.status == other.status


class Report(gasp.Deserializable):
    title: str
    sections: List[str]
    
    def __eq__(self, other):
        if not isinstance(other, Report):
            return False
        return self.title == other.title and self.sections == other.sections


class Answer(gasp.Deserializable):
    value: str
    unit: str
    
    def __eq__(self, other):
        if not isinstance(other, Answer):
            return False
        return self.value == other.value and self.unit == other.unit


def test_default_ignored_tags():
    """Test that default ignored tags (think/thinking/system) are filtered out"""
    # Simulate LLM output with thinking tags (ignored by default)
    llm_output = """
<think>
Let me think about this question...
The capital of France is Paris.
</think>

<Result>
<answer type="string">The capital of France is Paris</answer>
<confidence type="float">0.95</confidence>
</Result>

<thinking>
I'm very confident about this answer.
</thinking>
"""
    
    # Parse only the Result tag (think/thinking/system tags are ignored by default)
    parser = gasp.Parser(Result)
    result = parser.feed(llm_output)
    
    assert result is not None
    assert isinstance(result, Result)
    assert result.answer == "The capital of France is Paris"
    assert result.confidence == 0.95


def test_custom_ignored_tags():
    """Test custom ignored tags override defaults"""
    llm_output = """
<Response>
    <data type="string">Important data</data>
    <status type="string">success</status>
</Response>

<DebugInfo>
    <processing_time type="float">0.5</processing_time>
    <memory_used type="string">100MB</memory_used>
</DebugInfo>
"""
    
    # Parse with custom ignored tags (overrides defaults)
    parser = gasp.Parser(Response, ignored_tags=["DebugInfo"])
    result = parser.feed(llm_output)
    
    assert result is not None
    assert isinstance(result, Response)
    assert result.data == "Important data"
    assert result.status == "success"


def test_no_ignored_tags():
    """Test parsing with no ignored tags (empty list overrides defaults)"""
    llm_output = """
<Report>
    <title type="str">Monthly Report</title>
    <sections type="list[str]">
        <item>Introduction</item>
        <item>Analysis</item>
        <item>Conclusion</item>
    </sections>
</Report>
"""
    
    # Parse with no ignored tags (empty list overrides defaults)
    parser = gasp.Parser(Report, ignored_tags=[])
    result = parser.feed(llm_output)
    
    assert result is not None
    assert isinstance(result, Report)
    assert result.title == "Monthly Report"
    assert result.sections == ["Introduction", "Analysis", "Conclusion"]


def test_streaming_with_ignored_tags():
    """Test streaming parsing with ignored tags"""
    # Simulate streaming chunks
    chunks = [
        "<system>Processing request...</system>",
        "<Answer>",
        '<value type="string">42</value>',
        '<unit type="string">degrees</unit>',
        "</Answer>",
        "<think>That was easy!</think>"
    ]
    
    # Create parser (system and think are ignored by default)
    parser = gasp.Parser(Answer)
    
    # Feed chunks one by one
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)
    
    assert result is not None
    assert isinstance(result, Answer)
    assert result.value == "42"
    assert result.unit == "degrees"
    assert parser.is_complete()


def test_multiple_ignored_tags():
    """Test multiple custom ignored tags"""
    llm_output = """
<metadata>
    <timestamp>2024-01-01</timestamp>
</metadata>

<debug>
    <trace>Step 1 complete</trace>
</debug>

<Answer>
    <value type="string">Solution</value>
    <unit type="string">units</unit>
</Answer>

<logs>
    <entry>Processing complete</entry>
</logs>
"""
    
    # Parse with multiple custom ignored tags
    parser = gasp.Parser(Answer, ignored_tags=["metadata", "debug", "logs"])
    result = parser.feed(llm_output)
    
    assert result is not None
    assert isinstance(result, Answer)
    assert result.value == "Solution"
    assert result.unit == "units"


def test_ignored_tags_with_nested_content():
    """Test that ignored tags with nested content are properly skipped"""
    llm_output = """
<think>
    <step>First, I need to analyze the problem</step>
    <step>Then, I'll formulate a solution</step>
    <nested>
        <deep>Very deep thought</deep>
    </nested>
</think>

<Result>
    <answer type="string">Final answer</answer>
    <confidence type="float">0.99</confidence>
</Result>
"""
    
    parser = gasp.Parser(Result)
    result = parser.feed(llm_output)
    
    assert result is not None
    assert isinstance(result, Result)
    assert result.answer == "Final answer"
    assert result.confidence == 0.99


def test_case_sensitive_ignored_tags():
    """Test that ignored tags are case-sensitive"""
    llm_output = """
<Think>
    This should not be ignored (capital T)
</Think>

<think>
    This should be ignored (lowercase t)
</think>

<Result>
    <answer type="string">Test answer</answer>
    <confidence type="float">0.5</confidence>
</Result>
"""
    
    parser = gasp.Parser(Result)
    result = parser.feed(llm_output)
    
    # The parser should still work even if there's an unrecognized tag
    assert result is not None
    assert isinstance(result, Result)
    assert result.answer == "Test answer"
    assert result.confidence == 0.5


def test_empty_ignored_tag():
    """Test that empty ignored tags are handled correctly"""
    llm_output = """
<system></system>
<think/>

<Result>
    <answer type="string">Empty tags test</answer>
    <confidence type="float">0.75</confidence>
</Result>

<thinking>

</thinking>
"""
    
    parser = gasp.Parser(Result)
    result = parser.feed(llm_output)
    
    assert result is not None
    assert isinstance(result, Result)
    assert result.answer == "Empty tags test"
    assert result.confidence == 0.75


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
