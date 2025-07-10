import gasp
import pytest


class CDataModel(gasp.Deserializable):
    content: str


def test_cdata_parsing():
    """
    Tests that the parser correctly handles a CDATA section and extracts its content as a raw string.
    """
    llm_output = """
    <CDataModel>
      <content><![CDATA[This is some text with <tags> and & symbols that should be ignored.]]></content>
    </CDataDataModel>
    """

    parser = gasp.Parser(CDataModel)
    result = parser.feed(llm_output)

    assert result is not None
    assert isinstance(result, CDataModel)
    assert (
        result.content
        == "This is some text with <tags> and & symbols that should be ignored."
    )


def test_cdata_split_across_chunks():
    """
    Tests that the parser handles CDATA sections split across multiple streaming chunks.
    """
    chunks = [
        "<CDataModel><content><![CDATA[Part 1, ",
        "Part 2, ",
        "and Part 3 with <stuff>]]></content></CDataModel>",
    ]

    parser = gasp.Parser(CDataModel)
    result = None
    for chunk in chunks:
        result = parser.feed(chunk)

    assert result is not None
    assert isinstance(result, CDataModel)
    assert result.content == "Part 1, Part 2, and Part 3 with <stuff>"
