(defmacro when (condition body)
  (list 'if condition body nil))

(let () (print "wow"))

; (defmacro when (condition body)
;   (list 'if condition body))

; (when (= 2 2) (print "wow")) TODO: This causes the error span to be here instead of inside the macro
