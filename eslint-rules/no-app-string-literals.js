function isModuleSpecifier(node) {
  const parent = node.parent;
  return (
    parent !== undefined &&
    (parent.type === 'ImportDeclaration' ||
      parent.type === 'ExportNamedDeclaration' ||
      parent.type === 'ExportAllDeclaration') &&
    parent.source === node
  );
}

function isDirective(node) {
  const parent = node.parent;
  return parent !== undefined && parent.type === 'ExpressionStatement' && parent.expression === node;
}

export default {
  meta: {
    type: 'problem',
    docs: {
      description: 'Disallow runtime string literals in app source files.',
    },
    messages: {
      runtimeString:
        'Runtime app source cannot own string literals. Move text, routes, ids, fields, and protocol values into a domain package.',
    },
    schema: [],
  },
  create(context) {
    return {
      Literal(node) {
        if (typeof node.value !== 'string' || isModuleSpecifier(node) || isDirective(node)) {
          return;
        }
        context.report({ node, messageId: 'runtimeString' });
      },
      TemplateLiteral(node) {
        for (const quasi of node.quasis) {
          const raw = quasi.value.raw;
          if (raw.length > 0) {
            context.report({ node: quasi, messageId: 'runtimeString' });
          }
        }
      },
    };
  },
};
