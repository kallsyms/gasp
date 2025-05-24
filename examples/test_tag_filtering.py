#!/usr/bin/env python3
"""
Test that the parser only looks for specific tags
"""

from gasp import Deserializable, Parser
from typing import List, Optional

# Define a simple class for testing
class Person(Deserializable):
    """A person with name and age"""
    name: str
    age: int
    
    def __repr__(self):
        return f"Person(name='{self.name}', age={self.age})"

def test_specific_tag():
    """Test that parser only parses content within the tag matching its type name"""
    print("=== Testing tag filtering ===")
    
    # Create a parser for the Person type
    parser = Parser(Person)
    
    # Test with correct tag
    correct_tag = '''<Person>{"name": "Alice", "age": 30}</Person>'''
    result1 = parser.feed(correct_tag)
    print(f"Correct tag parsing: {result1}")
    print(f"Is complete: {parser.is_complete()}")
    
    # Reset parser for next test
    parser = Parser(Person)
    
    # Test with nested tags including a <think> tag
    mixed_tags = '''
    <think>
        This is a thinking section that should be ignored by the parser.
        {"name": "Ignored", "age": 99}
    </think>
    <Person>
        {"name": "Bob", "age": 25}
    </Person>
    <other>This should also be ignored</other>
    '''
    
    result2 = parser.feed(mixed_tags)
    print(f"Mixed tags parsing: {result2}")
    print(f"Is complete: {parser.is_complete()}")
    
    # Verify only the correct tag's content was parsed
    assert result2 is not None, "Parser should have found a result"
    assert result2.name == "Bob", f"Expected 'Bob' but got '{result2.name}'"
    assert result2.age == 25, f"Expected 25 but got {result2.age}"
    print("Mixed tags test passed!")

if __name__ == "__main__":
    test_specific_tag()
