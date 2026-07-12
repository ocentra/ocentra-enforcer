"use strict";
/**
 * Safe counterparts for CYBER-NOSQL-INJECT.1 — every request-derived value
 * is explicitly cast/validated (or replaced with a local variable/static
 * literal) before it reaches a MongoDB query filter, so an attacker cannot
 * substitute an operator object for the string the query expects.
 */
const express = require("express");

const router = express.Router();
let db;

// (1) No $where at all: the request value is cast to a string and used in
// an ordinary field comparison instead of server-side JS.
router.get("/search", (req, res) => {
  const term = String(req.query.term);
  db.collection("logs")
    .find({ description: term })
    .toArray((err, docs) => res.json(docs));
});

// (2) Static $where string literal, no variable at all — left clean since
// there is nothing here for an attacker to influence.
router.get("/search-legacy", (req, res) => {
  db.collection("users")
    .find({ $where: "this.active == true" })
    .toArray((err, docs) => res.json(docs));
});

// (3) Only individually cast fields are read out of req.body — never the
// whole request object is passed into a query method.
router.post("/login", (req, res) => {
  const username = String(req.body.username);
  const password = String(req.body.password);
  db.collection("users").findOne({ username, password }, (err, user) => {
    if (user) return res.json({ token: "issued" });
    res.status(401).json({ error: "invalid credentials" });
  });
});

// (4) Each query parameter is cast/validated before use; the request
// object itself is never passed directly into a query method.
router.get("/admin/users", (req, res) => {
  const role = String(req.query.role || "member");
  db.collection("users")
    .find({ role })
    .toArray((err, docs) => res.json(docs));
});

// (5) Explicit String() cast closes off the {"$gt": ""}-shaped payload.
router.post("/profile", (req, res) => {
  db.collection("users")
    .find({ username: String(req.body.username) })
    .toArray((err, docs) => res.json(docs));
});

// (6) parseInt()/Number() cast on a numeric filter value used with $eq.
router.get("/profile/:id", (req, res) => {
  db.collection("users")
    .find({ age: { $eq: parseInt(req.query.age, 10) } })
    .toArray((err, docs) => res.json(docs));
});

// (7) Static, allowlisted $regex pattern — not derived from request data.
router.get("/search-active", (req, res) => {
  db.collection("users")
    .find({ status: { $regex: "^active-" } })
    .toArray((err, docs) => res.json(docs));
});

// (8) No JSON.parse of raw request text: the filter is built from a
// validated, individually cast field instead.
router.get("/reports", (req, res) => {
  const category = String(req.query.category || "");
  db.collection("reports")
    .find({ category })
    .toArray((err, docs) => res.json(docs));
});

// (9) Mongoose-style schema-validated call: the request field is still
// cast before use, so it can never carry an operator object.
router.post("/orders", (req, res) => {
  db.collection("orders")
    .find({ orderId: String(req.body.orderId) })
    .toArray((err, docs) => res.json(docs));
});

module.exports = router;
