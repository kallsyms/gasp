"""Test the exact LLM response format from the user's code using interpolate_prompt"""

from typing import List, Union, Optional
from gasp.template_helpers import interpolate_prompt
from gasp.deserializable import Deserializable


class IssueForm(Deserializable):
    provider: str
    title: str
    description: str
    labels: List[str]
    assignees: List[str]
    bismuth_assigned: Optional[bool]
    status: str
    priority: str


class WaitForConfirmation(Deserializable):
    prompt: str


class ActOnConfirmable(Deserializable):
    data: Optional[str]


class Chat(Deserializable):
    content: str


class CodeTool(Deserializable):
    filename: str
    language: str
    content: str
    description: Optional[str]
    dependencies: Optional[List[str]]


class SaveKnowledge(Deserializable):
    title: str
    content: str
    category: Optional[str]
    tags: Optional[List[str]]
    importance: Optional[str]  # "low" | "medium" | "high"
    context: str
    source: Optional[str]


class MetaPlanItem(Deserializable):
    step: str
    thoughts: str
    tools: List[str]


class MetaPlan(Deserializable):
    title: Optional[str]
    summary: Optional[str]
    items: List[MetaPlanItem]
    reasoning: str


class PlanTask(Deserializable):
    goal: str


class Plan(Deserializable):
    """Mock Plan class"""
    steps: List[str]


# The exact type alias from the user's code
AgentAction = Union[Chat, IssueForm, CodeTool, SaveKnowledge, MetaPlan, WaitForConfirmation, ActOnConfirmable, Plan, PlanTask]

# The exact prompt template from the user
prompt_template = """The user has sent this message:
{{ user_message }}
----

Behaviour rules:
• Decide if the request requires planning and use the planning tool. Follow the planning tool guidelines below if so.
• When the user explicitly requests an external action that maps to a tool,
  respond **only** with the matching JSON object, streamed token-by-token.
• Otherwise reply in normal conversational text using the Chat tool.
• Never wrap JSON in markdown fences.

Available tools:
• Chat: For conversational responses
• IssueForm: For creating GitHub/Linear issues
• WaitForConfirmation: For waiting for the user to confirm an action
• ActOnConfirmable: For acting on a previously confirmed action
• CodeTool: For generating, editing, or displaying code files
• SaveKnowledge: For capturing and organizing important information
• MetaPlan: For orchestrating complex multi-step proccesses to achieve the user's objective.
• Plan: For creating a detailed, step-by-step plan for a software development task.
• PlanTask: For creating a detailed, step-by-step plan for a software development task.

Use CodeTool when the user:
• Asks you to write, create, or generate code
• Requests a specific file or script
• Wants to see code examples or implementations
• Asks for code modifications or improvements

**Confirmation Flow Rules:**
1. When you use a tool that requires confirmation (like `IssueForm`), you **MUST** use the `WaitForConfirmation` tool immediately after it in the same turn.
2. You **MUST NOT** use the `ActOnConfirmable` tool in the same turn as the tool that requires confirmation.
3. Wait for the user to respond with their confirmation before using the `ActOnConfirmable` tool in a subsequent turn.

Any time you need input from a user to confirm an action, use the `WaitForConfirmation` tool. This includes things like creating an issue with `IssueForm`.
After the user confirms, use the `ActOnConfirmable` tool to proceed with the action.

Use SaveKnowledge when you want to:
• Capture important insights, learnings, or solutions discovered during conversation
• Document key technical concepts, patterns, or best practices that emerged
• Save useful information that could benefit future conversations
• Record troubleshooting steps or debugging approaches that worked
• Organize domain knowledge, APIs, or configuration details
• Store contextual information about user preferences or project specifics

Use MetaPlan when the user's request:
• Involves multiple tools or multi-step coordination (code + issues + documentation)
• Complexity is high so planning will reduce potential use frustration by confirming their intents align with your understanding.
• Is explicitly asking for planning or strategy ("How would you approach this?")

IMPORTANT: If you use MetaPlan or Plan, do NOT call any other action tools (IssueForm, CodeTool) in the same response.
You may only use Chat and SaveKnowledge after MetaPlan or Plan. Present the plan and let the user review it before execution.

<active_entities>
{{ active_entities }}
</active_entities>


Outline some relevant information before you answer.

You're encouraged to generate multiple actions in a single response when it makes sense to provide the user context and complete multiple tasks in one go if needed.
Being conversational is important, but you should also be efficient and not repeat yourself unnecessarily.
Try to always include a "Chat" along with any other actions you generate.

Planning is encouraged for multi tool tasks from the user to reduce friction. Consider planning for anything above trivial scenarios.

You can mark explicit knowledge you want to capture using the SaveKnowledge tool.

Break up actions with Chat messages to provide context and explanations.
That is in your response, you should always include a "Chat" action with the content of your response to the user.

Example sequence:
Chat: "Sure thing I'll make the issue for you."
IssueForm // if the user asked for an issue to be created or a plan requires it
Chat: "Okay great blah blah blah
...

{{return_type}}
"""

# Test the interpolation
print("=== Testing interpolate_prompt with List[AgentAction] ===")
result = interpolate_prompt(prompt_template, List[AgentAction], format_tag="return_type")

# Check if the format instructions are included
print("\n=== Checking if format instructions are included ===")
if "Your response should be formatted as:" in result:
    print("✓ Format instructions found")
    # Extract just the format instructions part
    format_start = result.find("Your response should be formatted as:")
    print("\n" + "="*80)
    print(result[format_start:])
else:
    print("✗ Format instructions NOT found")
    print("\nFull result:")
    print(result)
