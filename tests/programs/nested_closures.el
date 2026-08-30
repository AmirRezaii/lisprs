(defun outer (x)
  (lambda ()
    (lambda ()
      x)))

(let* ((a (outer 42))
       (b (a)))
  (b))
