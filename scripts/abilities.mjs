// Collects every character's ability text into site/abilities.json for the
// simulator's script view: data/characters/<id>.json specs plus
// data/abilities/*.json maps. Keys are ids without underscores. server/build.rs
// collects the same for the web app: keep the rules identical.
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const key = (id) => id.replaceAll("_", "");
const abilities = {};
for (const file of readdirSync(join(root, "data/characters")).filter((f) => f.endsWith(".json"))) {
  const spec = JSON.parse(readFileSync(join(root, "data/characters", file), "utf8"));
  if (spec.id && spec.ability) abilities[key(spec.id)] = spec.ability;
}
for (const file of readdirSync(join(root, "data/abilities")).filter((f) => f.endsWith(".json"))) {
  for (const [id, text] of Object.entries(JSON.parse(readFileSync(join(root, "data/abilities", file), "utf8")))) {
    abilities[key(id)] = text;
  }
}
writeFileSync(join(root, "site/abilities.json"), JSON.stringify(abilities));
console.log(`site/abilities.json: ${Object.keys(abilities).length} abilities`);
