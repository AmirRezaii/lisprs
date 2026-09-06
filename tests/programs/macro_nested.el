(defmacro unless (condition body)
  (list 'if condition nil body))

(defmacro twice (expr)
  (list 'progn expr expr))

(twice (unless false 5))
