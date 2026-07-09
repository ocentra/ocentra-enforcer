(defun add-numbers (a b)
  (+ a b))

(defmacro my-macro (x)
  `(list ,x))

(require 'cl-lib)

(add-numbers 1 2)
