#!/usr/bin/env python3
"""Debug named union parsing issue"""

from typing import Union
from gasp import Parser, Deserializable


class A(Deserializable):
    name: str
    value_a: int


class B(Deserializable):
    title: str  
    value_b: float


# Named type alias
type NamedUnion = Union[A, B]


def test_named_union():
    parser = Parser(NamedUnion)
    print(f"Parser created for NamedUnion")
    
    xml_data = '''<NamedUnion type="A">
    <name type="str">Named Test</name>
    <value_a type="int">100</value_a>
</NamedUnion>'''
    
    print(f"\nParsing XML:\n{xml_data}")
    
    # Feed in chunks to see what happens
    chunks = [
        '<NamedUnion type="A">',
        '\n    <name type="str">Named Test</name>',
        '\n    <value_a type="int">100</value_a>',
        '\n</NamedUnion>'
    ]
    
    for i, chunk in enumerate(chunks):
        print(f"\nFeeding chunk {i}: {repr(chunk)}")
        result = parser.feed(chunk)
        print(f"Result: {result}")
        print(f"Is complete: {parser.is_complete()}")
    
    final_result = parser.validate()
    print(f"\nFinal result: {final_result}")
    print(f"Final result type: {type(final_result)}")
    
    if final_result:
        print(f"Result attributes: {vars(final_result)}")


if __name__ == "__main__":
    test_named_union()
