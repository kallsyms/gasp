#!/usr/bin/env python3
"""Debug what type names are extracted for container types"""

from gasp import Parser
from typing import Dict, Set, Tuple, List

# Create parsers and check internal type info
types_to_check = [
    dict,
    Dict[str, int],
    set,
    Set[int],
    tuple,
    Tuple[int, ...],
    list,
    List[int]
]

for type_obj in types_to_check:
    parser = Parser(type_obj)
    # The parser has an internal parser attribute
    if hasattr(parser, 'parser') and hasattr(parser.parser, 'type_info'):
        type_info = parser.parser.type_info
        if type_info:
            print(f"{type_obj}: name='{type_info.name}', kind='{type_info.kind}'")
        else:
            print(f"{type_obj}: No type_info")
    else:
        print(f"{type_obj}: Cannot access internal type_info")
