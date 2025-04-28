#!/usr/bin/env python3

from gasp import WAILGenerator

def main():
    # Create a validator
    generator = WAILGenerator()

    # Load a WAIL schema with an intentional typo
    with open("./examples/constrained.wail", "r") as f:
        ideal_wail_schema = f.read()


    wail_schema = ideal_wail_schema
    generator.load_wail(wail_schema)

    print("HERE")

    x, y, z = generator.get_prompt(task="abcd")

    print(x)

    # Validate the schema
    warnings, errors = generator.validate_wail()

    print("Validation Results:")
    print("\nWarnings:")
    for warning in warnings:
        print(f"- {warning}")

    print("\nErrors:")
    for error in errors:
        print(f"- {error}")

if __name__ == "__main__":
    main() 