import { Router } from "express";
import { listWidgets } from "./widgets";

const router = Router();

function handleWidgets(req, res) {
  res.json(listWidgets());
}

router.get("/widgets", handleWidgets);

export { router };
