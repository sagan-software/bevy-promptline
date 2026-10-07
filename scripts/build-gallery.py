#!/usr/bin/env python3
"""Build independent WebGL2 examples from the tracked gallery inventory."""
import argparse
import html
import json
import shutil
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main():
    """Validate bindings, build selected artifacts, and render run pages."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bindgen", type=Path, required=True)
    parser.add_argument("--only", action="append", default=[])
    args = parser.parse_args()
    inventory = json.loads((ROOT / "web/examples.json").read_text())
    names = {item["name"] for item in inventory}
    if set(args.only) - names:
        parser.error("--only must name an inventory example")
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
    version = next(
        item["version"] for item in packages
        if item["name"] == "wasm-bindgen"
    )
    actual = subprocess.check_output(
        [str(args.bindgen), "--version"], text=True
    ).strip()
    if actual != f"wasm-bindgen {version}":
        raise RuntimeError(f"Expected wasm-bindgen {version}; found {actual}")
    for item in inventory:
        name = item["name"]
        if args.only and name not in args.only:
            continue
        command = [
            "cargo", "build", "--example", name, "--target",
            "wasm32-unknown-unknown", "--profile", "wasm-release",
        ]
        if item["features"]:
            command += ["--features", ",".join(item["features"])]
        subprocess.run(command, cwd=ROOT, check=True)
        destination = ROOT / "site" / name
        destination.mkdir(parents=True, exist_ok=True)
        source = (ROOT / "target/wasm32-unknown-unknown/wasm-release"
                  / "examples" / f"{name}.wasm")
        subprocess.run([
            str(args.bindgen), "--target", "web", "--out-dir",
            str(destination), "--out-name", "app", str(source),
        ], check=True)
        shutil.copytree(
            ROOT / "assets", destination / "assets", dirs_exist_ok=True
        )
        page = (ROOT / "web/run.html").read_text()
        for key in ["name", "title", "description", "controls"]:
            page = page.replace(
                f"@@{key}@@", html.escape(item[key], quote=True)
            )
        (destination / "index.html").write_text(page)
    shutil.copy(ROOT / "web/gallery.css", ROOT / "site/gallery.css")
    cards = "\n".join(
        (f'<li><a href="./{item["name"]}/">'
         f'<h2>{html.escape(item["title"])}</h2>'
         f'<p>{html.escape(item["description"])}</p></a></li>')
        for item in inventory
    )
    page = (ROOT / "web/index.html").read_text().replace("@@cards@@", cards)
    (ROOT / "site/index.html").write_text(page)
    (ROOT / "site/.nojekyll").touch()


if __name__ == "__main__":
    main()
