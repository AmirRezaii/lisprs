(defmacro when (condition body)
  `(if ,condition ,body nil))

(when true (print "wow"))
