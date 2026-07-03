import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const RULES_PATH = fileURLToPath(new URL('./source-policy-rules.json', import.meta.url));
const SOURCE_POLICY_RULES = JSON.parse(fs.readFileSync(RULES_PATH, 'utf8'));

export { SOURCE_POLICY_RULES };
