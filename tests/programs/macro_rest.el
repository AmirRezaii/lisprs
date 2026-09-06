(defmacro sum-at-least-one (first . rest)
  (cons '+ (cons first rest)))

(+ (sum-at-least-one 1) (sum-at-least-one 2 3 4))
