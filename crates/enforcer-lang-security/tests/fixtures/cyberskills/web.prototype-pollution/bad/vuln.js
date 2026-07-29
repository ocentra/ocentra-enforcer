// Settings/profile API backed by a hand-rolled recursive merge, plus
// lodash/jQuery deep-merge helpers fed directly from request input.
const _ = require("lodash");
const express = require("express");

const router = express.Router();

// Hand-rolled recursive merge: no __proto__/constructor/prototype denylist
// anywhere in this function or file, so a crafted source object pollutes
// every object's shared prototype.
function merge(target, source) {
  for (const key in source) {
    if (typeof source[key] === "object" && source[key] !== null) {
      target[key] = merge(target[key] || {}, source[key]);
    } else {
      target[key] = source[key];
    }
  }
  return target;
}

router.post("/settings", (req, res) => {
  const config = merge({}, req.body);
  res.json(config);
});

router.post("/profile", (req, res) => {
  // lodash deep merge fed directly from the request body.
  const profile = _.merge({}, req.body);
  res.json(profile);
});

router.post("/prefs", (req, res) => {
  const prefs = _.defaultsDeep({}, JSON.parse(req.query.raw));
  res.json(prefs);
});

router.post("/legacy", (req, res) => {
  jQuery.extend(true, {}, req.body);
});

router.post("/widgets", (req, res) => {
  $.extend(true, {}, req.body);
});

router.post("/bulk", (req, res) => {
  Object.assign({}, JSON.parse(req.body.raw));
});

// Direct prototype-write gadgets (the sinks the deep-merge helpers above
// reach when fed a `__proto__`/`constructor.prototype`-keyed payload).
function pollute(obj) {
  obj.__proto__.isAdmin = true;
  obj["__proto__"]["role"] = "admin";
  const Base = obj.constructor.prototype;
  Base.polluted = true;
}

module.exports = { router, merge, pollute };
