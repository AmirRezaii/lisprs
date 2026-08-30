(defun make-adder (x)
  (lambda (y)
    (+ x y)))

(let* ((add10 (make-adder 10)))
  (add10 32))
