#!/usr/bin/env python3
"""
Test the recursive model_dump functionality
"""

import pytest
from gasp import Deserializable
from typing import List, Optional


class Child(Deserializable):
    name: str
    age: int
    
    def __eq__(self, other):
        if not isinstance(other, Child):
            return False
        return self.name == other.name and self.age == other.age


class Parent(Deserializable):
    name: str
    child: Child
    children: List[Child]
    
    def __eq__(self, other):
        if not isinstance(other, Parent):
            return False
        return (self.name == other.name and 
                self.child == other.child and 
                self.children == other.children)


def test_recursive_model_dump():
    """Test that model_dump recursively converts nested objects to dicts"""
    # Create nested objects
    parent = Parent.__gasp_from_partial__({
        "name": "Parent", 
        "child": {"name": "Charlie", "age": 5},
        "children": [{"name": "Alice", "age": 8}, {"name": "Bob", "age": 10}]
    })
    
    # Verify the objects were created correctly
    assert isinstance(parent, Parent)
    assert isinstance(parent.child, Child)
    assert isinstance(parent.children[0], Child)
    assert isinstance(parent.children[1], Child)
    
    # Test model_dump - this should convert everything to dicts
    dumped = parent.model_dump()
    
    # Verify that nested objects are now dicts
    assert isinstance(dumped, dict)
    assert isinstance(dumped['child'], dict)
    assert isinstance(dumped['children'], list)
    assert isinstance(dumped['children'][0], dict)
    assert isinstance(dumped['children'][1], dict)
    
    # Verify the content is correct
    assert dumped['name'] == 'Parent'
    assert dumped['child']['name'] == 'Charlie'
    assert dumped['child']['age'] == 5
    assert dumped['children'][0]['name'] == 'Alice'
    assert dumped['children'][0]['age'] == 8
    assert dumped['children'][1]['name'] == 'Bob'
    assert dumped['children'][1]['age'] == 10


def test_model_dump_with_none_values():
    """Test model_dump with None values"""
    class OptionalChild(Deserializable):
        name: str
        nickname: Optional[str] = None
        age: int = 0
    
    child = OptionalChild.__gasp_from_partial__({"name": "Test"})
    dumped = child.model_dump(exclude_none=False)
    
    assert dumped == {"name": "Test", "nickname": None, "age": 0}


def test_model_dump_with_lists():
    """Test model_dump with various list types"""
    class Container(Deserializable):
        strings: List[str]
        numbers: List[int]
        children: List[Child]
    
    container = Container.__gasp_from_partial__({
        "strings": ["a", "b", "c"],
        "numbers": [1, 2, 3],
        "children": [
            {"name": "Child1", "age": 5},
            {"name": "Child2", "age": 7}
        ]
    })
    
    dumped = container.model_dump()
    
    assert dumped["strings"] == ["a", "b", "c"]
    assert dumped["numbers"] == [1, 2, 3]
    assert len(dumped["children"]) == 2
    assert dumped["children"][0] == {"name": "Child1", "age": 5}
    assert dumped["children"][1] == {"name": "Child2", "age": 7}


def test_deeply_nested_model_dump():
    """Test model_dump with deeply nested structures"""
    class GrandChild(Deserializable):
        name: str
        toy: str
    
    class ChildWithGrandChild(Deserializable):
        name: str
        grandchild: GrandChild
    
    class GrandParent(Deserializable):
        name: str
        child: ChildWithGrandChild
    
    grandparent = GrandParent.__gasp_from_partial__({
        "name": "GrandParent",
        "child": {
            "name": "Parent",
            "grandchild": {
                "name": "GrandChild",
                "toy": "teddy bear"
            }
        }
    })
    
    dumped = grandparent.model_dump()
    
    assert isinstance(dumped, dict)
    assert isinstance(dumped["child"], dict)
    assert isinstance(dumped["child"]["grandchild"], dict)
    assert dumped["child"]["grandchild"]["name"] == "GrandChild"
    assert dumped["child"]["grandchild"]["toy"] == "teddy bear"


def test_model_dump_empty_list():
    """Test model_dump with empty lists"""
    class EmptyContainer(Deserializable):
        name: str
        items: List[Child] = []
    
    container = EmptyContainer.__gasp_from_partial__({"name": "Empty"})
    dumped = container.model_dump()
    
    assert dumped == {"name": "Empty", "items": []}


def test_model_dump_with_dict_attributes():
    """Test model_dump with dict attributes"""
    class WithDict(Deserializable):
        name: str
        metadata: dict
        tags: dict[str, str]
    
    obj = WithDict.__gasp_from_partial__({
        "name": "Test",
        "metadata": {"key1": "value1", "key2": 123},
        "tags": {"env": "prod", "version": "1.0"}
    })
    
    dumped = obj.model_dump()
    
    assert dumped["metadata"] == {"key1": "value1", "key2": 123}
    assert dumped["tags"] == {"env": "prod", "version": "1.0"}


def test_model_dump_preserves_types():
    """Test that model_dump preserves basic types correctly"""
    class MixedTypes(Deserializable):
        string_val: str
        int_val: int
        float_val: float
        bool_val: bool
        none_val: Optional[str] = None
    
    obj = MixedTypes.__gasp_from_partial__({
        "string_val": "test",
        "int_val": 42,
        "float_val": 3.14,
        "bool_val": True,
        "none_val": None
    })
    
    dumped = obj.model_dump(exclude_none=False)
    
    assert isinstance(dumped["string_val"], str)
    assert isinstance(dumped["int_val"], int)
    assert isinstance(dumped["float_val"], float)
    assert isinstance(dumped["bool_val"], bool)
    assert dumped["none_val"] is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
