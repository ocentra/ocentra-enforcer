(in-package :widget)
(require :other-package)

(defun helper (x)
  (+ x 1))

(defun area (shape)
  (other (helper shape)))
