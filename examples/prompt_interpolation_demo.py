#!/usr/bin/env python3
"""
Demonstration of prompt interpolation with complex multi-class unions and various input types.
This example shows how gasp's template_helpers module handles different type structures.
"""

from typing import Union, List, Dict, Optional, Literal, Tuple, Set
from gasp import Deserializable
from gasp.template_helpers import type_to_format_instructions, interpolate_prompt


# === Example 1: Tool System with Complex Union ===
class WebSearchTool(Deserializable):
    """Search the web for information.
    
    query: The search query to execute
    max_results: Maximum number of results to return
    """
    query: str
    max_results: int = 10
    include_snippets: bool = True


class CodeExecutionTool(Deserializable):
    """Execute code in a sandboxed environment.
    
    language: Programming language for the code
    code: The code to execute
    timeout: Execution timeout in seconds
    """
    language: Literal["python", "javascript", "rust"]
    code: str
    timeout: Optional[int] = None


class FileOperationTool(Deserializable):
    """Perform file operations.
    
    operation: Type of file operation
    path: File path to operate on
    content: Content for write operations
    """
    operation: Literal["read", "write", "delete"]
    path: str
    content: Optional[str] = None


class DataPoint(Deserializable):
    """A single data point in analysis."""
    value: float
    timestamp: str
    metadata: Optional[Dict[str, str]] = None


class DataAnalysisTool(Deserializable):
    """Analyze numerical data.
    
    data_points: List of data points to analyze
    analysis_type: Type of analysis to perform
    options: Additional analysis options
    """
    data_points: List[DataPoint]
    analysis_type: Literal["mean", "median", "trend", "correlation"]
    options: Dict[str, Union[str, int, float, bool]]


# Create a complex union type
ToolUnion = Union[WebSearchTool, CodeExecutionTool, FileOperationTool, DataAnalysisTool]


# === Example 2: Response System with Nested Types ===
class Citation(Deserializable):
    """A citation reference."""
    source: str
    url: Optional[str]
    page: Optional[int]


class Section(Deserializable):
    """A section of content."""
    title: str
    content: str
    citations: List[Citation]
    subsections: Optional[List['Section']] = None


class ResearchResponse(Deserializable):
    """A detailed research response.
    
    topic: The research topic
    summary: Brief summary of findings
    sections: Detailed sections with citations
    confidence: Confidence score
    """
    topic: str
    summary: str
    sections: List[Section]
    confidence: float
    metadata: Dict[str, str]


class ErrorResponse(Deserializable):
    """An error response.
    
    error_type: Type of error that occurred
    message: Error message
    details: Additional error details
    """
    error_type: Literal["validation", "execution", "timeout", "unknown"]
    message: str
    details: Optional[Dict[str, str]] = None


class SimpleResponse(Deserializable):
    """A simple text response."""
    text: str
    tokens_used: int


# Another complex union
ResponseUnion = Union[ResearchResponse, ErrorResponse, SimpleResponse]


# === Example 3: Configuration with Container Types ===
class ModelConfig(Deserializable):
    """Model configuration settings."""
    model_name: str
    temperature: float
    max_tokens: int
    stop_sequences: List[str]
    
    
class CacheConfig(Deserializable):
    """Cache configuration."""
    enabled: bool
    ttl_seconds: int
    max_size_mb: Optional[int]


class SystemConfig(Deserializable):
    """Complete system configuration.
    
    models: Model configurations by name
    cache: Cache configuration
    features: Enabled features
    tags: System tags
    supported_formats: Tuple of supported formats
    """
    models: Dict[str, ModelConfig]
    cache: CacheConfig
    features: List[Literal["streaming", "caching", "logging", "metrics"]]
    tags: Optional[Set[str]]
    supported_formats: Tuple[str, ...]  # Homogeneous tuple


# === Example 4: Mixed Container Types ===
class DataStructures(Deserializable):
    """Examples of various container types."""
    string_list: List[str]
    int_tuple: Tuple[int, int, int]
    mixed_tuple: Tuple[str, int, bool]
    string_set: Set[str]
    nested_dict: Dict[str, List[int]]
    optional_list: Optional[List[str]]
    union_list: List[Union[str, int]]


# === Example 5: Type Aliases and Named Unions ===
type NamedToolUnion = Union[WebSearchTool, CodeExecutionTool]
type ComplexResponse = Union[ResearchResponse, ErrorResponse]
type ConfigValue = Union[str, int, float, bool, List[str], Dict[str, str]]


# === Complex Multi-Level Union ===
ComplexUnion = Union[ToolUnion, ResponseUnion, SystemConfig, DataStructures]


def main():
    """Demonstrate format instructions for various types."""
    
    print("=" * 80)
    print("GASP Prompt Interpolation Demo")
    print("=" * 80)
    
    # === Basic Types ===
    print("\n### BASIC TYPES ###")
    
    print("\n1. String:")
    print(type_to_format_instructions(str, name="query"))
    
    print("\n2. Integer:")
    print(type_to_format_instructions(int, name="count"))
    
    print("\n3. Float:")
    print(type_to_format_instructions(float, name="temperature"))
    
    print("\n4. Boolean:")
    print(type_to_format_instructions(bool, name="enabled"))
    
    # === Container Types ===
    print("\n\n### CONTAINER TYPES ###")
    
    print("\n1. List[str]:")
    print(type_to_format_instructions(List[str], name="tags"))
    
    print("\n2. Dict[str, int]:")
    print(type_to_format_instructions(Dict[str, int], name="scores"))
    
    print("\n3. Tuple[str, int, bool]:")
    print(type_to_format_instructions(Tuple[str, int, bool], name="config"))
    
    print("\n4. Set[str]:")
    print(type_to_format_instructions(Set[str], name="unique_tags"))
    
    print("\n5. Optional[int]:")
    print(type_to_format_instructions(Optional[int], name="timeout"))
    
    # === Classes ===
    print("\n\n### CLASSES ###")
    
    print("\n1. Simple Class (Citation):")
    print(type_to_format_instructions(Citation))
    
    print("\n2. Class with Nested List (Section):")
    print(type_to_format_instructions(Section))
    
    print("\n3. Complex Class (SystemConfig):")
    print(type_to_format_instructions(SystemConfig))
    
    # === Union Types ===
    print("\n\n### UNION TYPES ###")
    
    print("\n1. Tool Union (4 tools):")
    print(type_to_format_instructions(ToolUnion, name="tool"))
    
    print("\n2. Response Union:")
    print(type_to_format_instructions(ResponseUnion, name="response"))
    
    print("\n3. Named Type Alias Union:")
    print(type_to_format_instructions(NamedToolUnion, name="action"))
    
    print("\n4. Complex Multi-Union:")
    print(type_to_format_instructions(ComplexUnion, name="data"))
    
    # === Prompt Interpolation Examples ===
    print("\n\n### PROMPT INTERPOLATION ###")
    
    # Example 1: Tool Selection
    tool_template = """You are an AI assistant with access to various tools.

Select the appropriate tool based on the user's request:

{{tool_format}}

Guidelines:
- Choose the most appropriate tool
- Provide all required parameters
- Use defaults for optional parameters"""
    
    print("\n1. Tool Selection Prompt:")
    print(interpolate_prompt(tool_template, ToolUnion, format_tag="tool_format"))
    
    # Example 2: Response Generation
    response_template = """Generate a response in one of these formats:

{{response_format}}

Choose the format based on:
- ResearchResponse: for detailed, cited responses
- ErrorResponse: when errors occur
- SimpleResponse: for brief answers"""
    
    print("\n\n2. Response Generation Prompt:")
    print(interpolate_prompt(response_template, ResponseUnion, format_tag="response_format"))
    
    # Example 3: Configuration
    config_template = """Create a configuration:

{{config}}

Ensure all fields are valid."""
    
    print("\n\n3. Configuration Prompt:")
    print(interpolate_prompt(config_template, SystemConfig, format_tag="config"))
    
    # Example 4: Complex nested structures
    nested_template = """Handle the following data structure:

{{data_structure}}

This can be a tool, response, configuration, or data structure."""
    
    print("\n\n4. Complex Union Prompt:")
    print(interpolate_prompt(nested_template, ComplexUnion, format_tag="data_structure"))
    
    # === Edge Cases ===
    print("\n\n### EDGE CASES ###")
    
    print("\n1. Empty Class:")
    class EmptyClass(Deserializable):
        pass
    print(type_to_format_instructions(EmptyClass))
    
    print("\n2. Deeply Nested Types:")
    ComplexNested = Dict[str, List[Union[Dict[str, str], List[int]]]]
    print(type_to_format_instructions(ComplexNested, name="nested"))
    
    print("\n3. Optional Union:")
    OptionalUnion = Optional[Union[str, int]]
    print(type_to_format_instructions(OptionalUnion, name="value"))
    
    print("\n4. List of Unions:")
    UnionList = List[Union[WebSearchTool, CodeExecutionTool]]
    print(type_to_format_instructions(UnionList, name="tools"))


if __name__ == "__main__":
    main()
