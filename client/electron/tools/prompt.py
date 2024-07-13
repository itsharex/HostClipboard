from pathlib import Path
import pyperclip

root_path = Path(__file__).parent.parent
print(f"{root_path=}")


def get_codefile_prompt(file: Path):
    with file.open() as f:
        code = f.read()

    # 相对于根目录的路径
    project_path = file.relative_to(root_path)

    return f"""```{str(project_path)}\n{code}```\n\n\n"""


if __name__ == '__main__':
    prompt = ""
    file_names = [
        "main.js",
        "preload.js",
        "pages/render.js",
        "pages/index.html",
        "pages/styles.css"
    ]
    for file_name in file_names:
        file = root_path / file_name
        prompt += get_codefile_prompt(file)
    pyperclip.copy(prompt)
