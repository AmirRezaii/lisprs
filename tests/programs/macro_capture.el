(defmacro add-via-capture (value)
  (let ((builder (lambda (x)
                   (list '+ x value))))
    (builder value)))

(add-via-capture 6)
