import pytest
import gasp
from typing import Union, Optional, Dict, List, Tuple


def test_primitives():
    """Test primitive type support: str, int, float, bool"""
    class Primitives:
        text: str
        number: int
        decimal: float
        flag: bool

    xml = '''<Primitives>
<text type="str">Hello</text>
<number type="int">42</number>
<decimal type="float">3.14</decimal>
<flag type="bool">true</flag>
</Primitives>'''

    result = gasp.Parser(Primitives).feed(xml)
    assert result is not None
    assert result.text == "Hello"
    assert result.number == 42
    assert result.decimal == 3.14
    assert result.flag is True


def test_lists():
    """Test list support"""
    class WithList(gasp.Deserializable):
        items: list[str]

    xml = '''<WithList>
<items type="list[str]">
    <item>A</item>
    <item>B</item>
    <item>C</item>
</items>
</WithList>'''

    result = gasp.Parser(WithList).feed(xml)
    assert result is not None
    assert result.items == ['A', 'B', 'C']


def test_nested_classes():
    """Test nested class support"""
    class Address(gasp.Deserializable):
        street: str
        city: str

    class Person(gasp.Deserializable):
        name: str
        address: Address

    xml = '''<Person>
<name type="str">John</name>
<address type="Address">
    <street type="str">123 Main St</street>
    <city type="str">Boston</city>
</address>
</Person>'''

    result = gasp.Parser(Person).feed(xml)
    assert result is not None
    assert result.name == "John"
    assert result.address.street == "123 Main St"
    assert result.address.city == "Boston"


def test_union_types():
    """Test union type support"""
    class Cat:
        meow: str

    class Dog:
        bark: str

    Animal = Union[Cat, Dog]

    # Test Cat
    xml_cat = '''<Cat>
<meow type="str">Meow!</meow>
</Cat>'''
    parser = gasp.Parser(Animal)
    result_cat = parser.feed(xml_cat)
    assert result_cat is not None
    assert isinstance(result_cat, Cat)
    assert result_cat.meow == "Meow!"

    # Test Dog
    xml_dog = '''<Dog>
<bark type="str">Woof!</bark>
</Dog>'''
    parser2 = gasp.Parser(Animal)
    result_dog = parser2.feed(xml_dog)
    assert result_dog is not None
    assert isinstance(result_dog, Dog)
    assert result_dog.bark == "Woof!"


def test_incremental_parsing():
    """Test incremental field parsing with partial values"""
    class Fruit:
        name: str

    parser = gasp.Parser(Fruit)
    chunks = ['<Fruit><name type="str">a', 'pp', 'le</name></Fruit>']
    values = []
    
    for chunk in chunks:
        result = parser.feed(chunk)
        if result and hasattr(result, 'name'):
            values.append(result.name)
    
    assert values == ['a', 'app', 'apple']
    assert parser.is_complete()


def test_type_support_summary():
    """Integration test covering all supported types"""
    # Test primitives
    test_primitives()
    
    # Test lists
    test_lists()
    
    # Test nested classes
    test_nested_classes()
    
    # Test unions
    test_union_types()
    
    # Test incremental parsing
    test_incremental_parsing()
    
    # All tests passed if we reach here
    assert True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
