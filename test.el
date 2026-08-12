(defun outer (y)
  (lambda (x)
    (+ x y)))

(defun three (z)
  (lambda (y)
    (lambda (x)
      (+ x y z))))

(((three 4) 3) 8)
