import gasp
from typing import List

def test_inner_list_parsing():
    """Debug inner list parsing step by step"""
    xml = """<list type="list[list[int]]">
    <item type="list[int]">
        <item type="int">1</item>
        <item type="int">2</item>
    </item>
</list>"""
    
    parser = gasp.Parser(List[List[int]])
    
    # Feed the entire XML at once
    result = parser.feed(xml)
    print("Full parse result:", result)
    print("Expected: [[1, 2]]")
    print("Match:", result == [[1, 2]])
    print()
    
    # Now test chunk by chunk
    chunks = [
        '<list type="list[list[int]]">',
        '    <item type="list[int]">',
        '        <item type="int">1</item>',
        '        <item type="int">2</item>',
        '    </item>',
        '</list>'
    ]
    
    parser2 = gasp.Parser(List[List[int]])
    for i, chunk in enumerate(chunks):
        result = parser2.feed(chunk)
        if result and len(result) > 0 and len(result[0]) > 0:
            print(f"After chunk {i}: {result}")
            if isinstance(result[0], list):
                print(f"  Inner list contents: {result[0]}")

if __name__ == "__main__":
    test_inner_list_parsing()
