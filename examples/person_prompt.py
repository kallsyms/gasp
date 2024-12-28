#!/usr/bin/env python3

from gasp import WAILValidator
import json

def create_person_prompt():
    """Define the WAIL schema for person generation."""
    return """
    # Define our Person type
    object Person {
        name: String
        age: Number
        interests: Array<String>
    }

    # Define a template for generating a person from a description
    template GetPersonFromDescription(description: String) -> Person {
        prompt: '''
        Given this description of a person: {{description}}
        Create a Person object with their name, age, and interests.
        Return in this format: {{return_type}}
        '''
    }

    # Main section defines what we want to do
    main {
        person_prompt = GetPersonFromDescription(
            description: "Alice is a 25-year-old software engineer who loves coding, AI, and hiking."
        )
        prompt {
            {{person_prompt}}
        }
    }
    """

def main():
    # Initialize our validator with the schema
    validator = WAILValidator()
    validator.load_wail(create_person_prompt())

    # Get the generated prompt - this is what you'd send to your LLM
    prompt = validator.get_prompt()
    print("Generated Prompt:")
    print(prompt)
    print()

    # In a real application, you would send this prompt to your LLM
    # Here we'll simulate an LLM response with some typical quirks
    llm_response = """
    {
        'name': 'Alice',  # Single quoted strings
        age: 25,          # Unquoted key and number
        'interests': [    # Mix of quote styles
            "coding",     # Double quotes
            'AI',         # Single quotes
            hiking,       # Unquoted string
        ]                 # GASP handles all these cases
    }
    """

    try:
        # Validate the LLM's response
        validator.validate_json(llm_response)
        print("✓ Response validation successful!")
        
        # Get the parsed and validated response as a Python dict
        result = validator.get_parsed_json()
        
        # Work with the validated data
        print("\nParsed Person:")
        print(f"Name: {result['name']}")
        print(f"Age: {result['age']}")
        print(f"Interests: {', '.join(result['interests'])}")
        
        # You can also convert it to standard JSON
        print("\nAs standard JSON:")
        print(json.dumps(result, indent=2))
        
    except Exception as e:
        print(f"❌ Validation error: {e}")

if __name__ == "__main__":
    main() 