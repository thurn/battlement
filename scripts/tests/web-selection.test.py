#!/usr/bin/env python3
"""Check the native/browser boundary and enforce complete declared site interactions."""

from pathlib import Path
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from web_selection import select
from web_compatibility import contracts, site_digest


def main():
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        (root / "web/checks").mkdir(parents=True)
        (root / "web/contracts.toml").write_text('''
[[risks]]
paths = ["web/init.js"]
reason = "Browser input and resizing"
samples = ["*"]
[[risks]]
paths = ["samples/basic/rules/src/platform.rs"]
reason = "Declared shared renderer platform risk"
samples = ["basic"]
''')
        names = ["basic", "chess"]
        assert select(root, ["samples/chess/rules/src/settings.rs", "docs/readme.md"], names) == {}
        assert set(select(root, ["web/init.js"], names)) == set(names)
        assert list(select(root, ["samples/basic/rules/src/platform.rs"], names)) == ["basic"]
        assert list(select(root, ["web/checks/chess.js"], names)) == ["chess"]
        assert set(select(root, ["web/contracts.toml"], names)) == set(names)
        for name in names:
            (root / f"web/checks/{name}.js").write_text("async () => ({})")
        assert list(contracts(root, names)) == names
        try:
            contracts(root, [*names, "new-shipped-sample"])
            raise AssertionError("An undeclared shipped sample was accepted")
        except RuntimeError:
            pass
        first = site_digest(root)
        (root / "web/checks/basic.js").write_text("changed")
        assert first != site_digest(root)
        (root / "link").symlink_to(root / "web/contracts.toml")
        try:
            site_digest(root)
            raise AssertionError("A mutable symlink was accepted as prepared site content")
        except RuntimeError:
            pass
    print("Browser selection boundary checks passed")


if __name__ == "__main__":
    main()
