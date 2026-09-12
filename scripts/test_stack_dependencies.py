"""Exercise Fihrist's edges without depending on live sibling checkouts."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "stack_dependencies", Path(__file__).with_name("check-stack-dependencies.py")
)
stack = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stack)


class FihristEdges(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        environment = {}
        versions = {"typesec": "0.14.0", "lakecat": "0.4.0", "fihrist": "0.1.0"}
        packages = {"typesec": "typesec-core", "lakecat": "lakecat-core"}
        for name in stack.ORDER:
            root = self.root / name
            root.mkdir()
            (root / "Cargo.toml").write_text(
                f'[package]\nname = "{packages.get(name, name)}"\n'
                f'version = "{versions.get(name, "0.1.0")}"\n'
            )
            environment[str(stack.REPOS[name]["env"])] = str(root)
        self.environment = patch.dict(os.environ, environment)
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.dependencies("fihrist", 'lakecat-core = "0.4.0"\ntypesec-core = "0.14.0"')

    def dependencies(self, consumer, dependencies):
        with (self.root / consumer / "Cargo.toml").open("a") as manifest:
            manifest.write(f"\n[dependencies]\n{dependencies}\n")

    def test_released_fihrist_edges_are_tracked_in_both_directions(self):
        self.dependencies("querygraph", 'fihrist = "0.1.0"')
        roots, versions, edges, problems, warnings = stack.collect()
        self.assertEqual(problems, [])
        self.assertEqual(warnings, [])
        self.assertEqual(
            {(edge[0], edge[3]) for edge in edges},
            {("querygraph", "fihrist"), ("fihrist", "lakecat"), ("fihrist", "typesec")},
        )
        self.assertLess(stack.ORDER.index("lakecat"), stack.ORDER.index("fihrist"))
        self.assertLess(stack.ORDER.index("fihrist"), stack.ORDER.index("querygraph"))
        self.assertIn("| querygraph | `Cargo.toml` | `fihrist` | fihrist |", stack.render(roots, versions, edges))

    def test_unreleased_source_and_stale_fihrist_pin_block_release(self):
        for requirement in ['"0.0.1"', '{ version = "0.1.0", path = "../fihrist" }']:
            with self.subTest(requirement=requirement):
                manifest = self.root / "querygraph" / "Cargo.toml"
                manifest.write_text(
                    '[package]\nname = "querygraph"\nversion = "0.1.0"\n'
                    f'[dependencies]\nfihrist = {requirement}\n'
                )
                *_, problems, warnings = stack.collect()
                self.assertEqual(len(problems), 1)
                self.assertIn("fihrist", problems[0])
                self.assertEqual(warnings, [])

    def test_stale_typesec_pin_in_fihrist_blocks_release(self):
        manifest = self.root / "fihrist" / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('"0.14.0"', '"0.13.0"'))
        *_, problems, warnings = stack.collect()
        self.assertEqual(len(problems), 1)
        self.assertIn("fihrist/Cargo.toml: typesec-core", problems[0])
        self.assertEqual(warnings, [])


if __name__ == "__main__":
    unittest.main()
