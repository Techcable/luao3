from pathlib import Path
import re

EXAMPLE_PATTERN = re.compile(r"^\s*##[#]*\s*Example[s]?", flags=re.IGNORECASE | re.MULTILINE)

CODE_BEGIN_PATTERN = re.compile(r"^```[`]?(\w+)?\s*$", flags=re.MULTILINE)
CODE_END_PATTERN = re.compile(r"^```[`]?\s*$", flags=re.MULTILINE)

README_PATH = Path("../README.md")


GENERATED_FILE_HEADER = [
    "// !!! WARNING !!!",
    "//",
    "// Automatically extracted from README.md by extract_example.py",
    "// Please do not edit directly"
]


def extract_example_code(expected_language='rust') -> list[str]:
    with open(README_PATH, 'rt') as f:
        text = f.read()
        header = EXAMPLE_PATTERN.search(text)
        assert header is not None, 'Could not find "## Example" header in README.md'
        examples = []
        index = header.end()
        while True:
            code_begin = CODE_BEGIN_PATTERN.search(text, index)
            if code_begin is None:
                break
            assert code_begin.group(1) in (expected_language, None), f"Unexpected language: {code_begin.group(1)!r}"
            code_end = CODE_END_PATTERN.search(text, code_begin.end() + 1)
            assert code_end is not None, "Could not find end of code block"
            code = text[code_begin.end():code_end.start()].strip()
            examples.append(code)
            index = code_end.end() + 1
        return examples


def format_example_tests(examples: list[str]) -> str:
    res = [*GENERATED_FILE_HEADER]
    for idx, example in enumerate(examples):
        res.extend(("", ""))
        res.append(f"mod example{idx+1} {{")
        res.extend(" " * 4 + line for line in example.splitlines())
        res.append("}")
    return '\n'.join(res)


def main():
    examples = extract_example_code()
    with open('../tests/readme.rs', 'wt') as f:
        f.write(format_example_tests(examples))


if __name__ == "__main__":
    main()
