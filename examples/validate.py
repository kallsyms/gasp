#!/usr/bin/env python3

from gasp import WAILValidator

def main():
    # Create a validator
    validator = WAILValidator()

    # Load a WAIL schema with an intentional typo
    wail_schema = '''
object Person {
    name: String
    age: Number
}

template GetPersonFromDescription(description: String) -> Person {
    prompt: """
    Given this description of a person: {{description}}
    Create a Person object with their name and age.
    Return in this format: {{return_type}}
    """
}
'''

    validator.load_wail(wail_schema)

    # Validate the schema
    warnings, errors = validator.validate()

    print("Validation Results:")
    print("\nWarnings:")
    for warning in warnings:
        print(f"- {warning}")

    print("\nErrors:")
    for error in errors:
        print(f"- {error}")

if __name__ == "__main__":
    main() 