"use strict";
/**
 * Fixtures for CYBER-NOSQL-INJECT.1 — realistic Express + MongoDB routes
 * where unsanitized request data flows straight into query filters.
 */
const express = require("express");

const router = express.Router();
let db;

// (1) $where built from a template-literal string with ${} interpolation.
// MongoDB executes $where server-side as JavaScript, so an attacker who
// controls `term` can inject arbitrary JS (SKILL.md Step 4).
router.get("/search", (req, res) => {
  const term = req.query.term;
  db.collection("logs")
    .find({ $where: `this.description.indexOf("${term}") !== -1` })
    .toArray((err, docs) => res.json(docs));
});

// (2) $where built via string concatenation with a variable.
router.get("/search-legacy", (req, res) => {
  const username = req.query.username;
  db.collection("users")
    .find({ $where: "this.username == " + username })
    .toArray((err, docs) => res.json(docs));
});

// (3) Raw req.body passed straight into .findOne() — an attacker can send
// a JSON body like {"username":{"$ne":""},"password":{"$ne":""}} to bypass
// authentication entirely (SKILL.md "Operator Injection"/"Auth Bypass").
router.post("/login", (req, res) => {
  db.collection("users").findOne(req.body, (err, user) => {
    if (user) return res.json({ token: "issued" });
    res.status(401).json({ error: "invalid credentials" });
  });
});

// (4) Raw req.query passed straight into .find().
router.get("/admin/users", (req, res) => {
  db.collection("users")
    .find(req.query)
    .toArray((err, docs) => res.json(docs));
});

// (5) A single un-cast req.body property used as a query-filter value.
// An attacker can send {"username":{"$gt":""}} as the JSON body to match
// any user (type juggling: object where a string is expected).
router.post("/profile", (req, res) => {
  db.collection("users")
    .find({ username: req.body.username })
    .toArray((err, docs) => res.json(docs));
});

// (6) $regex given a bare, un-cast request property — the boolean/blind
// extraction technique from SKILL.md Steps 3-4 and
// process.py::blind_extract_field.
router.post("/search-password", (req, res) => {
  db.collection("users")
    .find({ password: { $regex: req.body.pattern } })
    .toArray((err, docs) => res.json(docs));
});

// (7) JSON.parse of raw request text used to build a query object,
// bypassing the framework's normal body-parser typing entirely.
router.get("/reports", (req, res) => {
  const filter = JSON.parse(req.query.filter);
  db.collection("reports")
    .find(filter)
    .toArray((err, docs) => res.json(docs));
});

module.exports = router;
