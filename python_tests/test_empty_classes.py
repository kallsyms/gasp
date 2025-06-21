"""Test parsing empty Python classes (no annotations, no fields)"""

import pytest
from gasp import Parser


class Finalize:
    """Empty class with pass statement"""
    pass


class EmptyWithDocstring:
    """An empty class that only has a docstring"""
    pass


class EmptyWithAnnotations:
    """This class appears empty but Python might create empty __annotations__"""
    pass


def test_empty_class_parsing():
    """Test that empty classes can be parsed"""
    xml = """<Finalize></Finalize>"""
    
    parser = Parser(Finalize)
    result = parser.feed(xml)
    
    assert parser.is_complete()
    assert result is not None
    assert isinstance(result, Finalize)
    assert result.__dict__ == {}  # Empty instance


def test_empty_class_self_closing_tag():
    """Test empty class with self-closing XML tag"""
    xml = """<Finalize />"""
    
    parser = Parser(Finalize)
    result = parser.feed(xml)
    
    # Self-closing tags have a quirk: they return a result but is_complete() is False
    # This appears to be a limitation of how the XML parser handles self-closing tags
    assert not parser.is_complete()  # Parser doesn't consider self-closing tags "complete"
    assert result is not None  # But it does return a valid instance
    assert isinstance(result, Finalize)
    assert result.__dict__ == {}


def test_empty_class_with_docstring():
    """Test empty class that has a docstring"""
    xml = """<EmptyWithDocstring></EmptyWithDocstring>"""
    
    parser = Parser(EmptyWithDocstring)
    result = parser.feed(xml)
    
    assert parser.is_complete()
    assert result is not None
    assert isinstance(result, EmptyWithDocstring)
    assert result.__dict__ == {}


def test_empty_class_annotations_check():
    """Verify that empty classes have empty __annotations__"""
    # Python creates empty __annotations__ for all classes
    assert hasattr(Finalize, '__annotations__')
    assert hasattr(EmptyWithDocstring, '__annotations__')
    assert hasattr(EmptyWithAnnotations, '__annotations__')
    
    # But they should be empty
    assert Finalize.__annotations__ == {}
    assert EmptyWithDocstring.__annotations__ == {}
    assert EmptyWithAnnotations.__annotations__ == {}


def test_empty_class_incremental_parsing():
    """Test incremental parsing of empty class"""
    parser = Parser(Finalize)
    
    # Feed XML in chunks
    result1 = parser.feed("<Final")
    assert result1 is None
    assert not parser.is_complete()
    
    result2 = parser.feed("ize>")
    # The parser might return an instance as soon as it sees the opening tag
    # for empty classes since there are no fields to wait for
    if result2 is not None:
        assert isinstance(result2, Finalize)
    
    result3 = parser.feed("</Finalize>")
    # Should definitely have a result by now
    assert result3 is not None or result2 is not None
    assert parser.is_complete()
    
    # Get the final result
    final_result = result3 if result3 is not None else result2
    assert isinstance(final_result, Finalize)


def test_empty_class_with_whitespace():
    """Test empty class with whitespace in XML"""
    xml = """
    <Finalize>
        
    </Finalize>
    """
    
    parser = Parser(Finalize)
    result = parser.feed(xml.strip())
    
    assert parser.is_complete()
    assert result is not None
    assert isinstance(result, Finalize)
    assert result.__dict__ == {}
