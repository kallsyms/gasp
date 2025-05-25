"""
GASP - Type-Safe LLM Output Parser

GASP is a Rust-based parser for turning LLM outputs into properly typed Python objects.
It handles streaming JSON fragments, recovers from common LLM quirks, and makes structured 
data extraction actually pleasant.
"""

from . import template_helpers
from . import jinja_helpers
from .deserializable import Deserializable

# Import native components from the Rust module
from .gasp import Parser as _RustParser, StreamParser

# Default ignored tags
DEFAULT_IGNORED_TAGS = ["think", "thinking", "system"]

def Parser(type_obj=None, ignored_tags=None):
    """
    Create a Parser with Python-level defaults for ignored tags.
    
    Args:
        type_obj: The Python type to parse into
        ignored_tags: List of tag names to ignore. Defaults to ["think", "thinking", "system"]
    
    Returns:
        A Parser instance
    """
    if ignored_tags is None:
        ignored_tags = DEFAULT_IGNORED_TAGS
    return _RustParser(type_obj, ignored_tags)

# Import key Jinja helpers for convenience
from .jinja_helpers import render_template, render_file_template

__version__ = "1.0.0"
__all__ = [
    "Parser", 
    "StreamParser", 
    "Deserializable", 
    "template_helpers", 
    "jinja_helpers",
    "render_template",
    "render_file_template"
]
