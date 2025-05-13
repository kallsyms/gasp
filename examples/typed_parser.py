from gasp import Deserializable, Parser
from typing import List, Optional

class Address(Deserializable):
    """An address with street, city, and zip code"""
    street: str
    city: str
    zip_code: str

class Person(Deserializable):
    """A person with name, age, and address"""
    name: str
    age: int
    address: Address
    hobbies: Optional[List[str]] = None

def main():
    # Create a parser for the Person type
    parser = Parser(Person)
    
    # Feed chunks with proper tags to indicate types
    chunk1 = '<Person>{"name": "Alice", "age": 30'
    result1 = parser.feed(chunk1)
    print("Chunk 1 result:", result1)  # Person(name='Alice', age=30)
    
    chunk2 = ', "address": {"street": "123 Main St", "city": "Springfield"'
    result2 = parser.feed(chunk2)
    print("Chunk 2 result:", result2)  # Person with partial address
    
    chunk3 = ', "zip_code": "12345"}, "hobbies": ["reading", "coding"]}</Person>'
    result3 = parser.feed(chunk3)
    print("Chunk 3 result:", result3)  # Complete Person object
    
    # Check if parsing is complete
    print("Is complete:", parser.is_complete())  # True
    
    # Get validated result
    validated = parser.validate()
    print("Validated result:", validated)

def pydantic_example():
    try:
        from pydantic import BaseModel
    except ImportError:
        print("Pydantic not installed, skipping example")
        return
        
    class PydanticAddress(BaseModel):
        street: str
        city: str
        zip_code: str
        
    class PydanticPerson(BaseModel):
        name: str
        age: int
        address: PydanticAddress
        hobbies: Optional[List[str]] = None
    
    # Create parser from Pydantic model
    parser = Parser.from_pydantic(PydanticPerson)
    
    # Feed chunks with proper tags to indicate types  
    chunks = [
        '<PydanticPerson>{"name": "Bob", "age": 25',
        ', "address": {"street": "456 Oak Ave", "city": "Rivertown"',
        ', "zip_code": "67890"}, "hobbies": ["gaming", "hiking"]}</PydanticPerson>'
    ]
    
    for i, chunk in enumerate(chunks, 1):
        result = parser.feed(chunk)
        print(f"Chunk {i} result:", result)
    
    # Get validated result
    validated = parser.validate()
    print("Validated result:", validated)
    
    # Since we already have a dict, we can create a Pydantic object from it
    if validated:
        # Create a Pydantic model from the parsed dict
        pydantic_obj = PydanticPerson.model_validate(validated)
        # Now we can call model_dump() on the Pydantic object
        data = pydantic_obj.model_dump()
        print("As dict:", data)

if __name__ == "__main__":
    print("=== Basic example ===")
    main()
    print("\n=== Pydantic example ===")
    pydantic_example()
