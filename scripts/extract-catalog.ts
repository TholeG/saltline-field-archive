import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { parseSync, traverse, types as t } from "@babel/core";
import { patchGameAudio, patchGameGeometry } from "../engine/patch-game";
import { formatCatalogSources } from "./format-catalog-source";

// Reuse the local game's actual constructors and DSP; never start its game loop.
const root = resolve(import.meta.dir, "..");
const original = await Bun.file(resolve(root, "game.js")).text();
const marker = "cL().catch(n=>yT(n&&n.stack||n));";
if (original.split(marker).length !== 2) {
  throw new Error(
    "Der Spieleinstieg hat sich geändert. Extraktion bitte prüfen.",
  );
}
const formatted = new Bun.Transpiler({ loader: "js" }).transformSync(original);
const appStart = formatted.indexOf("  function BM(");
if (appStart < 0) throw new Error("Anwendungsfunktionen nicht gefunden.");
const names = [
  ...formatted.matchAll(/^  (?:async )?function\s*\*?\s*([\w$]+)\(/gm),
]
  .filter((match) => match.index! >= appStart)
  .map((match) => match[1]);
// Important mathematical helpers are arrow functions in the original bundle.
names.push(
  "ei",
  "ms",
  "qt",
  "it",
  "Dt",
  "H0",
  "Qr",
  "Ak",
  "sw",
  "a6",
  "l6",
  "h6",
  "d6",
  "p6",
  "m6",
);
if (names.length < 200) throw new Error("Unvollständige Funktionsextraktion.");
const locations = Object.fromEntries(
  names.map((name) => {
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match =
      new RegExp(
        "(?:async\\s+)?function(?:\\s+|\\s*\\*\\s*)" + escaped + "\\(",
      ).exec(original) ?? new RegExp("\\b" + escaped + "=").exec(original);
    if (!match) throw new Error("Quellfundstelle fehlt: " + name);
    const offset = match.index;
    return [
      name,
      {
        file: "game.js",
        line: original.slice(0, offset).split("\n").length,
        byte: new TextEncoder().encode(original.slice(0, offset)).length,
      },
    ];
  }),
);
const sha256 = new Bun.CryptoHasher("sha256").update(original).digest("hex");
const formattedSources = await formatCatalogSources(original, names);
const knownNames = new Set(names);
const dependencies = Object.fromEntries(
  Object.entries(formattedSources)
    .map(([name, code]) => [
      name,
      [
        ...new Set(
          [...code.matchAll(/([\w$]+)\s*\(/g)].map((match) => match[1]),
        ),
      ].filter((candidate) => candidate !== name && knownNames.has(candidate)),
    ])
    .filter(([, references]) => references.length > 0),
);
const bridge = `var saltlineGeometry, saltlineKernels;
globalThis.SaltlineCatalog = {
  THREE: ut, sizes: Yi, builders: uv, createProps: fv, createMaterials: lv,
  createEvents: BM, random: su, audioBank: Ir, synthesize: K6, synthesizeSteps: J6,
  functions: {Fh, Ei, vg, bn, Jn, ll},
  constants: { Yi, T_, wn, G0, Ky, N5, Og, Kn, QP, eI, ji },
  geometry: { rifle: TE, pistol: CS, hound: VS, gull: WS, skin: z5 },
  backdrop: kw, waves: Hw,
  setObjectGeometry(geometry) { saltlineGeometry = geometry; },
  setGameKernels(kernels) { saltlineKernels = kernels; },
  dsp: { noise: e0, filter: kt, sweep: ei, envelope: ms, mix: Se, saturate: fi },
  source: ${JSON.stringify({ file: "game.js", sha256, locations, dependencies })}
};`;

function parseEngine(source: string) {
  const ast = parseSync(source, { babelrc: false, configFile: false });
  const wrapper = ast?.program.body[0];
  if (
    !t.isExpressionStatement(wrapper) ||
    !t.isCallExpression(wrapper.expression) ||
    !t.isArrowFunctionExpression(wrapper.expression.callee) ||
    !t.isBlockStatement(wrapper.expression.callee.body)
  )
    throw new Error("Unbekannter Aufbau des Spiel-Bundles.");
  return { ast: ast!, block: wrapper.expression.callee.body };
}

async function buildEngine(source: string) {
  const { block } = parseEngine(source);
  const three = block.body.find(
    (statement) =>
      t.isExpressionStatement(statement) &&
      t.isCallExpression(statement.expression) &&
      t.isIdentifier(statement.expression.callee, { name: "ar" }) &&
      t.isIdentifier(statement.expression.arguments[0], { name: "ut" }),
  );
  if (
    !three ||
    !t.isExpressionStatement(three) ||
    !t.isCallExpression(three.expression)
  ) {
    throw new Error("Three.js-Namespace fehlt im Spiel-Bundle.");
  }
  const exports = three.expression.arguments[1];
  if (!t.isObjectExpression(exports))
    throw new Error("Unbekannte Three.js-Exporte.");
  const viewer = await Bun.file(resolve(root, "catalog/app.js")).text();
  const used = new Set([
    ...Array.from(source.matchAll(/\but\.([\w$]+)/g), ([, name]) => name),
    ...Array.from(viewer.matchAll(/\bT\.([\w$]+)/g), ([, name]) => name),
  ]);
  const properties = exports.properties.filter(
    (property) =>
      t.isObjectProperty(property) &&
      t.isIdentifier(property.key) &&
      used.has(property.key.name),
  );
  if (properties.length !== used.size)
    throw new Error("Ein Three.js-Export fehlt.");
  const threeExports = properties
    .map((property) => source.slice(property.start!, property.end!))
    .join(",");

  const body =
    source.slice(block.start! + 1, exports.start! + 1) +
    threeExports +
    source.slice(exports.end! - 1, block.end! - 1);

  // The entry point is the IIFE body so Bun can discard unreferenced declarations.
  const temporary = await mkdtemp(resolve(tmpdir(), "saltline-catalog-"));
  try {
    const input = resolve(temporary, "catalog-entry.js");
    async function bundle(entry: string) {
      await Bun.write(input, entry);
      const result = await Bun.build({
        entrypoints: [input],
        target: "browser",
        format: "iife",
        minify: { whitespace: true, syntax: true, identifiers: false },
      });
      if (!result.success)
        throw new AggregateError(result.logs, "Katalog-Build fehlgeschlagen.");
      return result.outputs[0].text();
    }
    // The first pass drops the game entry point and its references to private module namespaces.
    const bundled = await bundle(body);
    return (await bundle(pruneUnusedNamespaces(bundled))).replace(
      /[\t ]+$/gm,
      "",
    );
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

function pruneUnusedNamespaces(source: string) {
  const { ast, block } = parseEngine(source);
  const unused: t.ExpressionStatement[] = [];
  traverse(ast, {
    BlockStatement(path) {
      if (path.node !== block) return;
      for (const statement of block.body) {
        if (
          !t.isExpressionStatement(statement) ||
          !t.isCallExpression(statement.expression)
        )
          continue;
        const {
          callee,
          arguments: [namespace, map],
        } = statement.expression;
        if (
          !t.isIdentifier(callee, { name: "ar" }) ||
          !t.isIdentifier(namespace) ||
          !t.isObjectExpression(map)
        )
          continue;
        const references = path.scope.getBinding(
          namespace.name,
        )?.referencePaths;
        // A private namespace whose sole use is its export registration can be discarded.
        if (references?.length === 1 && references[0].node === namespace) {
          unused.push(statement);
        }
      }
      path.stop();
    },
  });
  let body = "";
  let cursor = block.start! + 1;
  for (const statement of unused) {
    body += source.slice(cursor, statement.start!);
    cursor = statement.end!;
  }
  return body + source.slice(cursor, block.end! - 1);
}

const engine = await buildEngine(
  patchGameAudio(patchGameGeometry(original)).replace(marker, bridge),
);
await Bun.write(
  resolve(root, "catalog/source-code.js"),
  `// Generated by bun run catalog:extract from game.js.\nexport const sourceHash = ${JSON.stringify(sha256)};\nexport const snippets = ${JSON.stringify(formattedSources, null, 2)};\n`,
);
await Bun.write(
  resolve(root, "catalog/engine.js"),
  `// Generated by bun run catalog:extract from game.js; SHA-256 ${sha256}\n${engine}`,
);
await Bun.write(
  resolve(root, "catalog/manifest.json"),
  JSON.stringify(
    {
      source: "game.js",
      sha256,
      functions: names.length,
      extraction:
        "Original constructors and sound generators; game startup replaced by catalog exports.",
      locations,
    },
    null,
    2,
  ) + "\n",
);
console.log(
  `Katalog extrahiert: ${names.length} Funktionen; SHA-256 ${sha256}`,
);
