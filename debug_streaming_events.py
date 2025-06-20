#!/usr/bin/env python3
"""Debug streaming events from TagFinder"""

from gasp import Parser
import logging

# Set up logging to see debug output
logging.basicConfig(level=logging.DEBUG)

def test_streaming_with_events():
    """Test streaming parsing with event debugging"""
    parser = Parser(str)
    chunks = [
        '<str>',
        'Hello, ',
        'streaming ',
        'world!',
        '</str>'
    ]
    
    print("Testing streaming string parsing with event tracking...")
    result = None
    for i, chunk in enumerate(chunks):
        print(f"\n=== Feeding chunk {i}: {repr(chunk)} ===")
        result = parser.feed(chunk)
        print(f"Result after chunk {i}: {result}")
        print(f"Parser complete: {parser.is_complete()}")
    
    print(f"\nFinal result: {result}")
    print(f"Final parser complete: {parser.is_complete()}")
    
    # Let's also try feeding the same content but in different chunks
    print("\n\n=== Testing different chunking ===")
    parser2 = Parser(str)
    chunks2 = ['<str>Hello, streaming world!</str>']
    
    for i, chunk in enumerate(chunks2):
        print(f"\n=== Feeding chunk {i}: {repr(chunk)} ===")
        result = parser2.feed(chunk)
        print(f"Result: {result}")
        print(f"Complete: {parser2.is_complete()}")

if __name__ == "__main__":
    test_streaming_with_events()
