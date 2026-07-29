import { Router } from "express";

const router = Router();

router.get("/widgets", (req, res) => {
  res.json([]);
});

function listWidgets() {
  return [];
}

export { router, listWidgets };
