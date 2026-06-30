const domainTypeNamePattern =
  /^(?:.*(?:Id|ID|Path|Key|Name|Hash|URL|Url|Type|Slug|Route|Label|Title|Description|Status|Version).*)$/u;

function containsStringKeyword(node) {
  if (node.type === 'TSStringKeyword') {
    return true;
  }
  if (node.type === 'TSIntersectionType' || node.type === 'TSUnionType') {
    return node.types.some((item) => containsStringKeyword(item));
  }
  if (node.type === 'TSTypeReference' && node.typeParameters !== undefined) {
    return node.typeParameters.params.some((item) => containsStringKeyword(item));
  }
  return false;
}

function containsManualBrand(node) {
  if (node.type !== 'TSIntersectionType') {
    return false;
  }

  const hasString = node.types.some((item) => item.type === 'TSStringKeyword');
  const hasManualBrand = node.types.some(
    (item) =>
      item.type === 'TSTypeLiteral' &&
      item.members.some((member) => {
        const key = member.key;
        return key !== undefined && key.type === 'Identifier' && key.name === '__brand';
      })
  );

  return hasString && hasManualBrand;
}

export default {
  meta: {
    type: 'problem',
    docs: {
      description: 'Disallow naked domain string aliases and manual string brands.',
    },
    messages: {
      nakedAlias:
        'Domain-bearing string aliases are not allowed. Use an Effect Schema brand and a decode helper instead.',
      manualBrand: 'Manual string brands are not allowed. Use Effect Schema brand construction instead.',
    },
    schema: [],
  },
  create(context) {
    return {
      TSTypeAliasDeclaration(node) {
        if (containsManualBrand(node.typeAnnotation)) {
          context.report({ node, messageId: 'manualBrand' });
          return;
        }

        if (domainTypeNamePattern.test(node.id.name) && containsStringKeyword(node.typeAnnotation)) {
          context.report({ node, messageId: 'nakedAlias' });
        }
      },
    };
  },
};
