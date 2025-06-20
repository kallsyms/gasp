import gasp
from typing import Dict, Tuple, Set

print("=== Testing Dict, Tuple, and Set support ===\n")

# Test 1: Dict
print("1. DICT:")
try:
    parser = gasp.Parser(Dict[str, int])
    xml = '''<dict type="dict[str, int]">
    <item key="a">1</item>
    <item key="b">2</item>
    <item key="c">3</item>
</dict>'''
    result = parser.feed(xml)
    print(f"  Result: {result}")
    print(f"  Type: {type(result)}")
    print(f"  Is dict: {isinstance(result, dict)}")
except Exception as e:
    print(f"  ❌ Error: {e}")

# Test 2: Tuple
print("\n2. TUPLE:")
try:
    parser = gasp.Parser(Tuple[str, int, float])
    xml = '''<tuple type="tuple[str, int, float]">
    <item type="str">hello</item>
    <item type="int">42</item>
    <item type="float">3.14</item>
</tuple>'''
    result = parser.feed(xml)
    print(f"  Result: {result}")
    print(f"  Type: {type(result)}")
    print(f"  Is tuple: {isinstance(result, tuple)}")
except Exception as e:
    print(f"  ❌ Error: {e}")

# Test 3: Set
print("\n3. SET:")
try:
    parser = gasp.Parser(Set[int])
    xml = '''<set type="set[int]">
    <item type="int">1</item>
    <item type="int">2</item>
    <item type="int">3</item>
</set>'''
    result = parser.feed(xml)
    print(f"  Result: {result}")
    print(f"  Type: {type(result)}")
    print(f"  Is set: {isinstance(result, set)}")
except Exception as e:
    print(f"  ❌ Error: {e}")

print("\n=== Summary ===")
print("These types are recognized by the type system but don't have")
print("XML parsing implementations, which is why they show as warnings.")
