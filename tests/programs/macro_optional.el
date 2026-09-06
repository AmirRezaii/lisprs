(defmacro add-default (value (other value))
  (list '+ value other))

(defmacro literal-or-default ((value 10))
  (list 'quote value))

(+ (add-default 4) (literal-or-default) (literal-or-default 7))
