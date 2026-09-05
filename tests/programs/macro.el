(defmacro when (condition body)
  (list 'if condition body nil))

(when (= 2 2) 23)
