(define (greet name)
  (display "hi ")
  (draw name))

(define (draw name)
  (display name))

(struct point (x y))

(greet "world")
