#!/usr/bin/env python3
"""Test parsing of the LLM response."""

from typing import List, Union
from gasp import Deserializable, Parser

# Define the types from the response
class Chat(Deserializable):
    content: str

class IssueForm(Deserializable):
    title: str
    description: str

class WaitForConfirmation(Deserializable):
    message: str

# Define the other types that were in the original union
class CodeTool(Deserializable):
    command: str
    language: str

class SaveKnowledge(Deserializable):
    content: str
    tags: List[str]

class MetaPlan(Deserializable):
    steps: List[str]

class ActOnConfirmable(Deserializable):
    action: str

class Plan(Deserializable):
    tasks: List[str]

class PlanTask(Deserializable):
    task_id: str
    description: str

# Define the response type
ResponseType = List[Union[Chat, IssueForm, CodeTool, SaveKnowledge, MetaPlan, WaitForConfirmation, ActOnConfirmable, Plan, PlanTask]]

def test_llm_response():
    """Test parsing the actual LLM response."""
    
    # The actual response from the LLM
    xml_response = """<List>
  <item type="Chat">
    <content>Got it. Apologies for the back-and-forth. I'll create the issue exactly as you specified.</content>
  </item>
  <item type="IssueForm">
    <title>frontend bug</title>
    <description>we have a frontend bug</description>
  </item>
  <item type="WaitForConfirmation">
    <message>Ready to create an issue with title 'frontend bug' and description 'we have a frontend bug'. Shall I proceed?</message>
  </item>
</List>"""
    
    print("=== Testing LLM Response Parsing ===\n")
    print("XML Response:")
    print(xml_response)
    print("\n" + "="*50 + "\n")
    
    # Create parser and feed the XML
    parser = Parser(ResponseType)
    try:
        result = parser.feed(xml_response)
        print(f"Parsed result: {result}")
        print(f"Result type: {type(result)}")
        print(f"Is list: {isinstance(result, list)}")
        
        if isinstance(result, list):
            print(f"Number of items: {len(result)}")
            for i, item in enumerate(result):
                print(f"\nItem {i}:")
                print(f"  Type: {type(item).__name__}")
                print(f"  Content: {item}")
                
                # Check the content
                if isinstance(item, Chat):
                    print(f"  Chat content: {item.content}")
                elif isinstance(item, IssueForm):
                    print(f"  Issue title: {item.title}")
                    print(f"  Issue description: {item.description}")
                elif isinstance(item, WaitForConfirmation):
                    print(f"  Confirmation message: {item.message}")
        
        print(f"\n✅ SUCCESS: The XML response parsed correctly!")
        
    except Exception as e:
        print(f"\n❌ ERROR: Failed to parse the response")
        print(f"Error type: {type(e).__name__}")
        print(f"Error message: {str(e)}")
        import traceback
        traceback.print_exc()

def test_streaming_response():
    """Test parsing the LLM response with streaming chunks."""
    
    # The actual response from the LLM
    xml_response = """<List>
  <item type="Chat">
    <content>Got it. Apologies for the back-and-forth. I'll create the issue exactly as you specified.</content>
  </item>
  <item type="IssueForm">
    <title>frontend bug</title>
    <description>we have a frontend bug</description>
  </item>
  <item type="WaitForConfirmation">
    <message>Ready to create an issue with title 'frontend bug' and description 'we have a frontend bug'. Shall I proceed?</message>
  </item>
</List>"""
    
    print("\n\n=== Testing Streaming LLM Response ===\n")
    
    # Test with different chunk sizes - pseudo-fuzzing
    import random
    random.seed(42)  # For reproducibility
    
    # Test fixed chunk sizes
    fixed_chunk_sizes = [1, 5, 10, 25, 50, 100, 200, 500]
    
    # Test random chunk sizes
    random_chunk_sizes = [random.randint(1, 100) for _ in range(10)]
    
    # Test edge cases: prime numbers (often good for finding boundary issues)
    prime_chunk_sizes = [7, 13, 17, 23, 29, 31, 37, 41, 43, 47]
    
    all_chunk_sizes = fixed_chunk_sizes + random_chunk_sizes + prime_chunk_sizes
    
    successes = 0
    failures = 0
    
    for test_num, chunk_size in enumerate(all_chunk_sizes, 1):
        print(f"\n--- Test {test_num}/{len(all_chunk_sizes)}: Chunk size {chunk_size} ---")
        
        # Create parser
        parser = Parser(ResponseType)
        
        # Split response into chunks
        chunks = []
        for i in range(0, len(xml_response), chunk_size):
            chunks.append(xml_response[i:i+chunk_size])
        
        print(f"Total chunks: {len(chunks)}")
        
        # Feed chunks one by one
        results_seen = []
        for i, chunk in enumerate(chunks):
            result = parser.feed(chunk)
            
            # Only print details for small chunk sizes or failures
            if chunk_size <= 10 or (result and not isinstance(result, list)):
                print(f"\nChunk {i+1}: {repr(chunk)}")
                if result:
                    if isinstance(result, list):
                        print(f"  Current items: {len(result)}")
                    else:
                        print(f"  Result type: {type(result)}")
            
            if result and isinstance(result, list):
                results_seen.append(len(result))
        
        # Final validation
        final_result = parser.validate()
        if final_result and isinstance(final_result, list) and len(final_result) == 3:
            if chunk_size > 10:
                print(f"Items progression: {results_seen}")
            
            # Verify all items are correct type AND have correct attributes
            validation_errors = []
            
            # Check Chat item
            if not isinstance(final_result[0], Chat):
                validation_errors.append(f"Item 0 wrong type: {type(final_result[0]).__name__}")
            elif final_result[0].content != "Got it. Apologies for the back-and-forth. I'll create the issue exactly as you specified.":
                validation_errors.append(f"Chat content wrong: {final_result[0].content!r}")
            
            # Check IssueForm item
            if not isinstance(final_result[1], IssueForm):
                validation_errors.append(f"Item 1 wrong type: {type(final_result[1]).__name__}")
            else:
                if final_result[1].title != "frontend bug":
                    validation_errors.append(f"IssueForm title wrong: {final_result[1].title!r}")
                if final_result[1].description != "we have a frontend bug":
                    validation_errors.append(f"IssueForm description wrong: {final_result[1].description!r}")
            
            # Check WaitForConfirmation item
            if not isinstance(final_result[2], WaitForConfirmation):
                validation_errors.append(f"Item 2 wrong type: {type(final_result[2]).__name__}")
            elif final_result[2].message != "Ready to create an issue with title 'frontend bug' and description 'we have a frontend bug'. Shall I proceed?":
                validation_errors.append(f"WaitForConfirmation message wrong: {final_result[2].message!r}")
            
            if validation_errors:
                failures += 1
                print(f"❌ FAILED: Validation errors:")
                for error in validation_errors:
                    print(f"  - {error}")
            else:
                successes += 1
                print(f"✅ SUCCESS: Got {len(final_result)} items with correct types and attributes")
        else:
            failures += 1
            print(f"❌ FAILED: Got {type(final_result)} with {len(final_result) if isinstance(final_result, list) else 'N/A'} items")
    
    print(f"\n\n=== SUMMARY ===")
    print(f"Total tests: {len(all_chunk_sizes)}")
    print(f"Successes: {successes}")
    print(f"Failures: {failures}")
    print(f"Success rate: {successes/len(all_chunk_sizes)*100:.1f}%")


def test_random_split_fuzzing():
    """Test with completely random split points (not fixed chunk sizes)."""
    
    xml_response = """<List>
  <item type="Chat">
    <content>Got it. Apologies for the back-and-forth. I'll create the issue exactly as you specified.</content>
  </item>
  <item type="IssueForm">
    <title>frontend bug</title>
    <description>we have a frontend bug</description>
  </item>
  <item type="WaitForConfirmation">
    <message>Ready to create an issue with title 'frontend bug' and description 'we have a frontend bug'. Shall I proceed?</message>
  </item>
</List>"""
    
    print("\n\n=== Testing Random Split Point Fuzzing ===\n")
    
    import random
    random.seed(42)
    
    successes = 0
    failures = 0
    num_tests = 20
    
    for test_num in range(1, num_tests + 1):
        print(f"\n--- Random split test {test_num}/{num_tests} ---")
        
        # Create random split points
        num_splits = random.randint(5, 50)
        split_points = sorted([0] + [random.randint(1, len(xml_response)-1) for _ in range(num_splits)] + [len(xml_response)])
        
        # Remove duplicates while maintaining order
        split_points = list(dict.fromkeys(split_points))
        
        # Create chunks from split points
        chunks = []
        for i in range(len(split_points) - 1):
            chunk = xml_response[split_points[i]:split_points[i+1]]
            if chunk:  # Only add non-empty chunks
                chunks.append(chunk)
        
        print(f"Random chunks: {len(chunks)}, sizes: {[len(c) for c in chunks[:10]]}{'...' if len(chunks) > 10 else ''}")
        
        # Create parser and feed chunks
        parser = Parser(ResponseType)
        
        for chunk in chunks:
            parser.feed(chunk)
        
        # Validate
        final_result = parser.validate()
        if final_result and isinstance(final_result, list) and len(final_result) == 3:
            # Check both types and attribute values
            validation_errors = []
            
            # Check Chat
            if not isinstance(final_result[0], Chat):
                validation_errors.append(f"Wrong type at 0: {type(final_result[0]).__name__}")
            elif final_result[0].content != "Got it. Apologies for the back-and-forth. I'll create the issue exactly as you specified.":
                validation_errors.append(f"Chat.content mismatch")
            
            # Check IssueForm
            if not isinstance(final_result[1], IssueForm):
                validation_errors.append(f"Wrong type at 1: {type(final_result[1]).__name__}")
            else:
                if final_result[1].title != "frontend bug":
                    validation_errors.append(f"IssueForm.title mismatch: {final_result[1].title!r}")
                if final_result[1].description != "we have a frontend bug":
                    validation_errors.append(f"IssueForm.description mismatch: {final_result[1].description!r}")
            
            # Check WaitForConfirmation
            if not isinstance(final_result[2], WaitForConfirmation):
                validation_errors.append(f"Wrong type at 2: {type(final_result[2]).__name__}")
            elif final_result[2].message != "Ready to create an issue with title 'frontend bug' and description 'we have a frontend bug'. Shall I proceed?":
                validation_errors.append(f"WaitForConfirmation.message mismatch")
            
            if validation_errors:
                failures += 1
                print(f"❌ FAILED: {', '.join(validation_errors)}")
            else:
                successes += 1
                print("✅ SUCCESS")
        else:
            failures += 1
            print(f"❌ FAILED: Wrong result structure")
    
    print(f"\n\n=== RANDOM SPLIT SUMMARY ===")
    print(f"Total tests: {num_tests}")
    print(f"Successes: {successes}")
    print(f"Failures: {failures}")
    print(f"Success rate: {successes/num_tests*100:.1f}%")

if __name__ == "__main__":
    test_llm_response()
    test_streaming_response()
    test_random_split_fuzzing()
