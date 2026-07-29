/** Checks whether source defines a conversion for the named DTO. */
export function hasDomainConversionFor(source, dtoName) {
  const escapedName = dtoName.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const conversionFromDto = new RegExp(
    `\\bimpl(?:\\s*<[^>{}]+>)?\\s+(?:core::convert::|std::convert::)?(?:TryFrom|From)\\s*<\\s*&?(?:'[_A-Za-z][_A-Za-z0-9]*\\s+)?${escapedName}\\b`,
    "u",
  );
  if (conversionFromDto.test(source)) return true;
  const conversionIntoDto = new RegExp(
    `\\bimpl(?:\\s*<[^>{}]+>)?\\s+(?:core::convert::|std::convert::)?(?:TryFrom|From)\\s*<[^>{}]+>\\s+for\\s+${escapedName}\\b`,
    "u",
  );
  if (conversionIntoDto.test(source)) return true;
  const mapperTakingDto = new RegExp(
    `\\bfn\\s+(?:map_to_domain|into_domain|to_domain)\\b[^({;]*\\([^)]*:\\s*&?(?:'[_A-Za-z][_A-Za-z0-9]*\\s+)?${escapedName}\\b`,
    "u",
  );
  if (mapperTakingDto.test(source)) return true;
  const inherentDomainMethod = new RegExp(
    `\\bimpl(?:\\s*<[^>{}]+>)?\\s+${escapedName}\\b[\\s\\S]*?\\bfn\\s+(?:into_domain|to_domain)\\b`,
    "u",
  );
  return inherentDomainMethod.test(source);
}

/** Checks whether source defines a separate domain counterpart for a DTO. */
export function hasSeparateDomainCounterpart(source, dtoName) {
  const domainName = dtoName.replace(/(?:Dto|DTO|Request|Response|Envelope)$/u, "");
  if (!domainName || domainName === dtoName) return false;
  const escapedDomainName = domainName.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const localDeclaration = new RegExp(
    `(?:^|\\n)\\s*(?:pub(?:\\([^)]*\\))?\\s+)?(?:struct|enum|type|trait)\\s+${escapedDomainName}\\b`,
    "u",
  );
  if (localDeclaration.test(source)) return true;
  const directImport = new RegExp(
    `(?:^|\\n)\\s*use\\s+[^;{]*::${escapedDomainName}\\s*;`,
    "u",
  );
  if (directImport.test(source)) return true;
  const groupedImport = new RegExp(
    `(?:^|\\n)\\s*use\\s+[^;]*\\{[^}]*\\b${escapedDomainName}\\b[^}]*\\}\\s*;`,
    "u",
  );
  return groupedImport.test(source);
}
