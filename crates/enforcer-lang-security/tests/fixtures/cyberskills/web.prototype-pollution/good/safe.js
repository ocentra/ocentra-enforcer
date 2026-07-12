// Settings/profile API using a guarded recursive merge and safe
// alternatives to plain-object prototype-chain writes.
const _ = require("lodash");
const express = require("express");

const router = express.Router();

// Guarded recursive merge: explicitly skips the three dangerous key names
// before ever touching target[key], and builds the target with
// Object.create(null) so it has no prototype to pollute in the first place.
function safeMerge(target, source) {
  for (const key in source) {
    if (key === "__proto__" || key === "constructor" || key === "prototype") {
      continue;
    }
    if (typeof source[key] === "object" && source[key] !== null) {
      target[key] = safeMerge(target[key] || Object.create(null), source[key]);
    } else {
      target[key] = source[key];
    }
  }
  return target;
}

router.post("/settings", (req, res) => {
  const config = safeMerge(Object.create(null), req.body);
  res.json(config);
});

router.post("/profile", (req, res) => {
  // Static, literal defaults only — no request-derived source object, so
  // there is nothing an attacker can shape into a __proto__ payload.
  const profile = _.merge({}, { theme: "dark", locale: "en-US" });
  res.json(profile);
});

router.post("/prefs", (req, res) => {
  // Map keys never touch the object prototype chain, unlike plain-object
  // computed-key writes.
  const allowed = new Map([
    ["a", 1],
    ["b", 2],
  ]);
  res.json(Object.fromEntries(allowed));
});

// Belt-and-suspenders: freeze the shared prototype so even a missed
// computed-key write elsewhere in the process cannot add properties to it.
Object.freeze(Object.prototype);

module.exports = { router, safeMerge };
