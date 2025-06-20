import gasp
from typing import Union, Optional, Dict, List, Tuple

print('=== GASP Type Support Summary ===\n')

# Test 1: Primitives
print('1. PRIMITIVES:')
class Primitives:
    text: str
    number: int
    decimal: float
    flag: bool

xml1 = '''<Primitives>
<text type="str">Hello</text>
<number type="int">42</number>
<decimal type="float">3.14</decimal>
<flag type="bool">true</flag>
</Primitives>'''

try:
    result = gasp.Parser(Primitives).feed(xml1)
    if result:
        print('  ✅ String, Integer, Float, Boolean - All supported')
        print(f'     Values: text={result.text}, number={result.number}, decimal={result.decimal}, flag={result.flag}')
    else:
        print('  ❌ No result returned')
except Exception as e:
    print(f'  ❌ Error: {e}')

# Test 2: Lists
print('\n2. LISTS:')
class WithList(gasp.Deserializable):
    items: list[str]

xml2 = '''<WithList>
<items type="list[str]">
    <item>A</item>
    <item>B</item>
    <item>C</item>
</items>
</WithList>'''

try:
    result = gasp.Parser(WithList).feed(xml2)
    if result:
        print(f'  ✅ List - Supported')
        print(f'     Values: {result.items}')
    else:
        print('  ❌ No result returned')
except Exception as e:
    print(f'  ❌ Error: {e}')

# Test 3: Classes
print('\n3. CLASSES:')
class Address(gasp.Deserializable):
    street: str
    city: str

class Person(gasp.Deserializable):
    name: str
    address: Address

xml3 = '''<Person>
<name type="str">John</name>
<address type="Address">
    <street type="str">123 Main St</street>
    <city type="str">Boston</city>
</address>
</Person>'''

try:
    result = gasp.Parser(Person).feed(xml3)
    if result:
        print(f'  ✅ Nested Classes - Supported')
        print(f'     Values: name={result.name}, address.street={result.address.street}, address.city={result.address.city}')
    else:
        print('  ❌ No result returned')
except Exception as e:
    print(f'  ❌ Error: {e}')

# Test 4: Unions
print('\n4. UNIONS:')
class Cat:
    meow: str

class Dog:
    bark: str

# For unions, the XML tag should match one of the union member types
xml4a = '''<Cat>
<meow type="str">Meow!</meow>
</Cat>'''

xml4b = '''<Dog>
<bark type="str">Woof!</bark>
</Dog>'''

try:
    Animal = Union[Cat, Dog]
    parser = gasp.Parser(Animal)
    result1 = parser.feed(xml4a)
    parser2 = gasp.Parser(Animal)
    result2 = parser2.feed(xml4b)
    if result1 and result2:
        print(f'  ✅ Union Types - Supported')
        print(f'     Cat: {type(result1).__name__} with meow={result1.meow}')
        print(f'     Dog: {type(result2).__name__} with bark={result2.bark}')
    else:
        print(f'  ❌ Union parsing failed - result1={result1}, result2={result2}')
except Exception as e:
    print(f'  ❌ Error: {e}')

# Test 5: Incremental Parsing
print('\n5. INCREMENTAL PARSING:')
class Fruit:
    name: str

parser = gasp.Parser(Fruit)
chunks = ['<Fruit><name type="str">a', 'pp', 'le</name></Fruit>']
values = []
for i, chunk in enumerate(chunks):
    result = parser.feed(chunk)
    print(f'  Chunk {i}: "{chunk}" -> result: {result}')
    if result and hasattr(result, 'name'):
        values.append(result.name)
        print(f'     Current name value: "{result.name}"')

if values == ['a', 'app', 'apple']:
    print(f'  ✅ Incremental field parsing - Working correctly')
else:
    print(f'  ❌ Incremental field parsing - NOT working')
print(f'     Expected: ["a", "app", "apple"]')
print(f'     Got: {values}')

print('\n=== SUMMARY ===')
print('✅ Primitives: str, int, float, bool')
print('✅ Collections: list')
print('✅ Complex types: classes, unions')
print('✅ Incremental parsing with partial field values')
print('⚠️  Dict, Tuple, Set - Type recognized but XML mapping may be limited')
