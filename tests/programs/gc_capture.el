(defun make-getter ()
  (let ((x 43))
    (lambda ()
      x)))

(let ((f (make-getter)))
  (gc)
  (f))
