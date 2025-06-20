"""Test incremental parsing for all types: dict, tuple, set, unions, and classes."""

import pytest
import gasp
from dataclasses import dataclass
from typing import Dict, Tuple, Set, Union, Optional


@dataclass
class Person(gasp.Deserializable):
    """Test class for incremental class parsing."""
    name: Optional[str] = None
    age: Optional[int] = None
    email: Optional[str] = None

    @classmethod
    def __gasp_from_partial__(cls, data: dict):
        print(f"Person.__gasp_from_partial__ called with keys: {list(data.keys())}")
        instance = cls()
        instance.name = data.get('name')
        instance.age = data.get('age')
        instance.email = data.get('email')
        print(f"  Created Person: name={instance.name}, age={instance.age}, email={instance.email}")
        return instance


@dataclass
class Address(gasp.Deserializable):
    """Another test class."""
    street: Optional[str] = None
    city: Optional[str] = None
    
    @classmethod
    def __gasp_from_partial__(cls, data: dict):
        print(f"Address.__gasp_from_partial__ called with keys: {list(data.keys())}")
        instance = cls()
        instance.street = data.get('street')
        instance.city = data.get('city')
        return instance


def test_incremental_dict():
    """Test that dicts are built incrementally."""
    xml = """<dict type="dict[str, int]">
<entry key="one" type="int">1</entry>
<entry key="two" type="int">2</entry>
<entry key="three" type="int">3</entry>
</dict>"""
    
    parser = gasp.Parser(Dict[str, int])
    chunk_size = 15
    entries_seen = []
    
    for i in range(0, len(xml), chunk_size):
        chunk = xml[i:i+chunk_size]
        print(f"\nChunk {i//chunk_size}: {repr(chunk)}")
        result = parser.feed(chunk)
        
        if result:
            print(f"  Dict has {len(result)} entries: {dict(result)}")
            if len(result) > len(entries_seen):
                entries_seen.append(len(result))
    
    # Should see incremental growth
    assert entries_seen == [1, 2, 3], f"Expected [1, 2, 3] but got {entries_seen}"
    

def test_incremental_set():
    """Test that sets are built incrementally."""
    xml = """<set type="set[str]">
<item type="str">apple</item>
<item type="str">banana</item>
<item type="str">cherry</item>
</set>"""
    
    parser = gasp.Parser(Set[str])
    chunk_size = 20
    sizes_seen = []
    
    for i in range(0, len(xml), chunk_size):
        chunk = xml[i:i+chunk_size]
        print(f"\nChunk {i//chunk_size}: {repr(chunk)}")
        result = parser.feed(chunk)
        
        if result:
            print(f"  Set has {len(result)} items: {result}")
            if len(result) > 0 and (not sizes_seen or len(result) > sizes_seen[-1]):
                sizes_seen.append(len(result))
    
    # Should see incremental growth
    assert sizes_seen == [1, 2, 3], f"Expected [1, 2, 3] but got {sizes_seen}"


def test_incremental_tuple():
    """Test that tuples are built incrementally."""
    xml = """<tuple type="tuple[int, str, float]">
<item type="int">42</item>
<item type="str">hello</item>
<item type="float">3.14</item>
</tuple>"""
    
    parser = gasp.Parser(Tuple[int, str, float])
    chunk_size = 20
    items_seen = []
    
    for i in range(0, len(xml), chunk_size):
        chunk = xml[i:i+chunk_size]
        print(f"\nChunk {i//chunk_size}: {repr(chunk)}")
        result = parser.feed(chunk)
        
        if result:
            # For tuples, check how many non-None items we have
            non_none_count = sum(1 for item in result if item is not None)
            print(f"  Tuple: {result}, non-None items: {non_none_count}")
            if non_none_count > 0 and (not items_seen or non_none_count > items_seen[-1]):
                items_seen.append(non_none_count)
    
    # Should see incremental growth
    assert items_seen == [1, 2, 3], f"Expected [1, 2, 3] but got {items_seen}"


def test_incremental_class():
    """Test that class instances are built incrementally."""
    xml = """<Person type="Person">
<name type="str">John Doe</name>
<age type="int">30</age>
<email type="str">john@example.com</email>
</Person>"""
    
    parser = gasp.Parser(Person)
    chunk_size = 25
    fields_populated = []
    
    for i in range(0, len(xml), chunk_size):
        chunk = xml[i:i+chunk_size]
        print(f"\nChunk {i//chunk_size}: {repr(chunk)}")
        result = parser.feed(chunk)
        
        if result:
            # Count populated fields
            populated = 0
            if result.name:
                populated += 1
            if result.age is not None:
                populated += 1
            if result.email:
                populated += 1
            
            print(f"  Person: name='{result.name}', age={result.age}, email='{result.email}'")
            print(f"  Populated fields: {populated}")
            
            if populated > 0 and (not fields_populated or populated > fields_populated[-1]):
                fields_populated.append(populated)
    
    # Should see incremental field population
    assert len(fields_populated) >= 2, f"Expected at least 2 incremental updates but got {fields_populated}"


def test_incremental_union():
    """Test that union types are detected and parsed incrementally."""
    PersonOrAddress = Union[Person, Address]
    
    # Test with Person
    xml1 = """<Person type="Person">
<name type="str">Jane Smith</name>
<age type="int">25</age>
</Person>"""
    
    parser1 = gasp.Parser(PersonOrAddress)
    chunk_size = 20
    person_fields = []
    
    for i in range(0, len(xml1), chunk_size):
        chunk = xml1[i:i+chunk_size]
        result = parser1.feed(chunk)
        
        if result and isinstance(result, Person):
            populated = sum(1 for v in [result.name, result.age] if v is not None)
            if populated > 0 and (not person_fields or populated > person_fields[-1]):
                person_fields.append(populated)
    
    assert len(person_fields) >= 1, f"Expected incremental Person updates but got {person_fields}"
    
    # Test with Address
    xml2 = """<Address type="Address">
<street type="str">123 Main St</street>
<city type="str">New York</city>
</Address>"""
    
    parser2 = gasp.Parser(PersonOrAddress)
    address_fields = []
    
    for i in range(0, len(xml2), chunk_size):
        chunk = xml2[i:i+chunk_size]
        result = parser2.feed(chunk)
        
        if result and isinstance(result, Address):
            populated = sum(1 for v in [result.street, result.city] if v is not None)
            if populated > 0 and (not address_fields or populated > address_fields[-1]):
                address_fields.append(populated)
    
    assert len(address_fields) >= 1, f"Expected incremental Address updates but got {address_fields}"


def test_nested_incremental():
    """Test incremental parsing of nested structures."""
    
    @dataclass
    class Team(gasp.Deserializable):
        name: Optional[str] = None
        members: Optional[list[Person]] = None
        
        @classmethod
        def __gasp_from_partial__(cls, data: dict):
            instance = cls()
            instance.name = data.get('name')
            instance.members = data.get('members', [])
            return instance
    
    xml = """<Team type="Team">
<name type="str">Engineering</name>
<members type="list[Person]">
<item type="Person">
  <name type="str">Alice</name>
  <age type="int">28</age>
</item>
<item type="Person">
  <name type="str">Bob</name>
  <age type="int">32</age>
</item>
</members>
</Team>"""
    
    parser = gasp.Parser(Team)
    chunk_size = 30
    states = []
    
    for i in range(0, len(xml), chunk_size):
        chunk = xml[i:i+chunk_size]
        result = parser.feed(chunk)
        
        if result:
            state = {
                'has_name': bool(result.name),
                'member_count': len(result.members) if result.members else 0
            }
            
            # Only record if state changed
            if not states or state != states[-1]:
                states.append(state)
                print(f"  State: name={result.name}, members={len(result.members) if result.members else 0}")
    
    # Should see incremental updates
    assert len(states) >= 2, f"Expected at least 2 state changes but got {states}"


if __name__ == "__main__":
    # Run with verbose output
    test_incremental_dict()
    print("\n" + "="*60 + "\n")
    
    test_incremental_set()
    print("\n" + "="*60 + "\n")
    
    test_incremental_tuple()
    print("\n" + "="*60 + "\n")
    
    test_incremental_class()
    print("\n" + "="*60 + "\n")
    
    test_incremental_union()
    print("\n" + "="*60 + "\n")
    
    test_nested_incremental()
