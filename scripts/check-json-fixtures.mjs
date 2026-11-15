import { readFile } from "node:fs/promises";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const cases = [
  ["docs/p4/adapters/capability-manifest-v1.schema.json", [
    "docs/p4/adapters/fixtures/hyprland-wayland-manifest.example.json",
    "docs/p4/adapters/fixtures/windows-uia-manifest.example.json",
  ]],
  ["docs/p4/context/context-package-v1.schema.json", [
    "docs/p4/context/fixtures/context-package-v1.example.json",
  ]],
  ["docs/p4/tools/tool-registry-v1.schema.json", [
    "docs/p4/tools/tool-registry-v1.example.json",
  ]],
];

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
for (const [schemaPath, fixturePaths] of cases) {
  const schema = JSON.parse(await readFile(schemaPath, "utf8"));
  const validate = ajv.compile(schema);
  for (const fixturePath of fixturePaths) {
    const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
    if (!validate(fixture)) {
      throw new Error(`${fixturePath}: ${ajv.errorsText(validate.errors, { separator: "; " })}`);
    }
    console.log(`valid: ${fixturePath}`);
  }
}
