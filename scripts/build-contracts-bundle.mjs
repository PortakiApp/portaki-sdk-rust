/**
 * Assemble le bundle de contrats d'une version du SDK.
 *
 * Les trois documents partent ensemble et c'est le point : un schéma de manifeste d'une version
 * avec des primitives d'une autre ne décrit aucun SDK réel. Le registre refuse d'ailleurs une
 * release à laquelle il en manque un.
 *
 *   node scripts/build-contracts-bundle.mjs 2.1.1 > bundle.json
 *
 * Sans argument, la version est lue dans le Cargo.toml de l'espace de travail — la même que
 * celle que `portaki build` tamponne dans un manifeste de module.
 */
import { readFile } from "node:fs/promises";
import { join } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;

/** Les trois contrats, avec le nom sous lequel le registre les range. */
const CONTRACTS = {
  "module.v1.json": "schema/module.v1.json",
  "host-ops.json": "contracts/host-ops.json",
  "sdui_primitives.json": "crates/portaki-sdk/sdui_primitives.json",
};

/** Version de l'espace de travail — la source dont dérivent toutes les crates publiées. */
async function workspaceVersion() {
  const manifest = await readFile(join(ROOT, "Cargo.toml"), "utf8");
  const found = manifest.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!found) {
    throw new Error("version introuvable dans Cargo.toml");
  }
  return found[1];
}

const requested = process.argv[2]?.trim();
const version = requested || (await workspaceVersion());

if (requested) {
  // Un tag qui ne correspond pas à l'espace de travail publierait des contrats sous une version
  // que personne ne tamponne — donc introuvables au moment de valider un module.
  const actual = await workspaceVersion();
  if (actual !== requested) {
    throw new Error(
      `le tag annonce ${requested} mais Cargo.toml porte ${actual} — aligne l'un sur l'autre`,
    );
  }
}

const contracts = {};
for (const [name, path] of Object.entries(CONTRACTS)) {
  const raw = await readFile(join(ROOT, path), "utf8");
  try {
    contracts[name] = JSON.parse(raw);
  } catch (failure) {
    throw new Error(`${path} n'est pas du JSON valide : ${failure.message}`);
  }
}

process.stdout.write(JSON.stringify({ version, channel: "stable", contracts }));
