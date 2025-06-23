import unittest
from typing import List, Optional
from gasp.deserializable import Deserializable
from gasp.template_helpers import interpolate_prompt
from typing import Union

class MetaPlanItem(Deserializable):
    step: str
    thoughts: str
    tools: List[str]

class MetaPlan(Deserializable):
    title: Optional[str]
    summary: Optional[str]
    items: List[MetaPlanItem]
    reasoning: str

class Chat(Deserializable):
    content: str

AgentAction = Union[MetaPlan, Chat]

class TestTemplateGeneration(unittest.TestCase):
    def test_nested_complex_type_in_list_with_union(self):
        """
        Tests that a complex type (MetaPlanItem) nested in a List (items)
        within a class (MetaPlan) that is part of a Union (AgentAction)
        gets a "When you see..." structure definition in the generated prompt.
        """
        template = "Please return an object of the following type:\n{{return_type}}"
        instructions = interpolate_prompt(template, List[AgentAction])
        
        # Check that the placeholder was replaced
        self.assertNotIn("{{return_type}}", instructions)

        # Check for the main List structure
        self.assertIn("<List type=\"list[MetaPlan | Chat]\">", instructions)
        
        # Check for the crucial MetaPlan and MetaPlanItem structure definitions
        self.assertIn("When you see 'MetaPlan' in a type attribute, use this structure:", instructions)
        self.assertIn("When you see 'MetaPlanItem' in a type attribute, use this structure:", instructions)
        
        # Check for the fields within the MetaPlanItem structure definition
        self.assertIn("<step type=\"str\">example string</step>", instructions)
        self.assertIn("<thoughts type=\"str\">example string</thoughts>", instructions)
        self.assertIn("<tools type=\"list[str]\">", instructions)

if __name__ == '__main__':
    unittest.main()
