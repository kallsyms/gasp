#!/usr/bin/env python3
from typing import Union, get_origin, get_args
import types

class A:
    name: str
    value_a: int

class B:
    title: str  
    value_b: float

# Named type alias
type NamedUnion = Union[A, B]

# Check what NamedUnion actually is
print(f"Type of NamedUnion: {type(NamedUnion)}")
print(f"Is it a TypeAliasType? {isinstance(NamedUnion, types.TypeAliasType) if hasattr(types, 'TypeAliasType') else 'N/A'}")
print(f"Has __value__? {hasattr(NamedUnion, '__value__')}")
if hasattr(NamedUnion, '__value__'):
    print(f"__value__: {NamedUnion.__value__}")
    print(f"get_origin(__value__): {get_origin(NamedUnion.__value__)}")
    print(f"get_args(__value__): {get_args(NamedUnion.__value__)}")

# Check all attributes
print("\nAll attributes of NamedUnion:")
for attr in sorted(dir(NamedUnion)):
    if not attr.startswith('_'):
        continue
    try:
        value = getattr(NamedUnion, attr)
        print(f"  {attr}: {value!r}")
    except:
        print(f"  {attr}: <error accessing>")
