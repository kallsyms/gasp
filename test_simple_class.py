"""Test parsing simple Python classes (not Deserializable or Pydantic)"""

from gasp import Parser

# Simple Python class with no base class
class SimpleUser:
    def __init__(self):
        self.name = ""
        self.age = 0
        self.email = ""

# Another simple class with type annotations
class AnnotatedUser:
    name: str
    age: int
    email: str
    
    def __init__(self):
        self.name = ""
        self.age = 0
        self.email = ""

# Class with optional __init__ parameters
class FlexibleUser:
    def __init__(self, name="", age=0, email=""):
        self.name = name
        self.age = age
        self.email = email

# Test parsing
def test_simple_class():
    print("Testing SimpleUser...")
    xml = """<SimpleUser>
    <name>John Doe</name>
    <age>30</age>
    <email>john@example.com</email>
</SimpleUser>"""
    
    try:
        parser = Parser(SimpleUser)
        result = parser.feed(xml)
        print(f"Is complete: {parser.is_complete()}")
        if result:
            print(f"Success! Parsed: name={result.name}, age={result.age}, email={result.email}")
        else:
            print("No result returned")
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()

def test_annotated_class():
    print("\nTesting AnnotatedUser...")
    xml = """<AnnotatedUser>
    <name>Jane Doe</name>
    <age>25</age>
    <email>jane@example.com</email>
</AnnotatedUser>"""
    
    try:
        parser = Parser(AnnotatedUser)
        result = parser.feed(xml)
        print(f"Is complete: {parser.is_complete()}")
        if result:
            print(f"Success! Parsed: name={result.name}, age={result.age}, email={result.email}")
        else:
            print("No result returned")
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()

def test_flexible_class():
    print("\nTesting FlexibleUser...")
    xml = """<FlexibleUser>
    <name>Bob Smith</name>
    <age>35</age>
    <email>bob@example.com</email>
</FlexibleUser>"""
    
    try:
        parser = Parser(FlexibleUser)
        result = parser.feed(xml)
        print(f"Is complete: {parser.is_complete()}")
        if result:
            print(f"Success! Parsed: name={result.name}, age={result.age}, email={result.email}")
        else:
            print("No result returned")
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_simple_class()
    test_annotated_class()
    test_flexible_class()
