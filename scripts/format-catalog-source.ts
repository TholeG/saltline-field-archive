import { parseSync, transformSync, types as t, type PluginObject } from "@babel/core";
import { format } from "prettier";

const babelOptions = {
  babelrc: false,
  configFile: false,
  sourceType: "script",
} as const;

function isSplittableSequence(
  node: t.Node | null | undefined,
): node is t.SequenceExpression {
  // A string moved to the front of a block could become a directive.
  return (
    t.isSequenceExpression(node) &&
    !node.expressions.some((expression) => t.isStringLiteral(expression))
  );
}

const expandStatements: PluginObject = {
  visitor: {
    ExpressionStatement(path) {
      const { expression } = path.node;
      if (isSplittableSequence(expression)) {
        const statements = expression.expressions.map((item) =>
          t.expressionStatement(item),
        );
        if (path.inList) path.replaceWithMultiple(statements);
        else path.replaceWith(t.blockStatement(statements));
      } else if (
        t.isLogicalExpression(expression) &&
        (expression.operator === "&&" || expression.operator === "||")
      ) {
        const condition =
          expression.operator === "&&"
            ? expression.left
            : t.unaryExpression("!", expression.left);
        path.replaceWith(
          t.ifStatement(
            condition,
            t.blockStatement([t.expressionStatement(expression.right)]),
          ),
        );
      }
    },
    ReturnStatement(path) {
      const { argument } = path.node;
      if (!isSplittableSequence(argument)) return;
      const expressions = argument.expressions;
      const statements = expressions
        .slice(0, -1)
        .map<t.Statement>((item) => t.expressionStatement(item));
      statements.push(t.returnStatement(expressions.at(-1)));
      if (path.inList) path.replaceWithMultiple(statements);
      else path.replaceWith(t.blockStatement(statements));
    },
    VariableDeclaration(path) {
      const { kind, declarations } = path.node;
      if (!path.inList || !path.parentPath.isBlockStatement()) return;
      if (declarations.length > 1) {
        path.replaceWithMultiple(
          declarations.map((declaration) =>
            t.variableDeclaration(kind, [declaration]),
          ),
        );
      }
    },
  },
};

export async function formatCatalogSources(original: string, names: string[]) {
  const wrapper = parseSync(original, babelOptions)?.program.body[0];
  if (
    !t.isExpressionStatement(wrapper) ||
    !t.isCallExpression(wrapper.expression) ||
    !t.isArrowFunctionExpression(wrapper.expression.callee) ||
    !t.isBlockStatement(wrapper.expression.callee.body)
  ) {
    throw new Error("Unbekannter Aufbau des Spiel-Bundles.");
  }

  const functions = new Map<
    string,
    t.FunctionDeclaration | t.ArrowFunctionExpression
  >();
  for (const statement of wrapper.expression.callee.body.body) {
    if (t.isFunctionDeclaration(statement) && statement.id) {
      functions.set(statement.id.name, statement);
    } else if (t.isVariableDeclaration(statement)) {
      for (const { id, init } of statement.declarations) {
        if (t.isIdentifier(id) && t.isArrowFunctionExpression(init)) {
          functions.set(id.name, init);
        }
      }
    }
  }

  const sources: Record<string, string> = {};
  for (const name of names) {
    const source = functions.get(name);
    if (source?.start == null || source.end == null) {
      throw new Error("Quellfunktion fehlt: " + name);
    }
    const text = original.slice(source.start, source.end);
    const expanded = transformSync(text, {
      ...babelOptions,
      plugins: [expandStatements],
    })?.code;
    if (!expanded) throw new Error("Leere formatierte Quellfunktion: " + name);
    sources[name] = (await format(expanded, { parser: "babel" })).trimEnd();
  }
  return sources;
}
