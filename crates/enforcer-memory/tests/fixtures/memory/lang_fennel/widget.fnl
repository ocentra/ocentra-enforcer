(fn helper [label]
  (print label))

(fn draw [name]
  (if (= name "")
      (helper "unnamed")
      (helper name)))

(local w (draw "world"))
