(setq f
  (let* ((x 0))
    (let* ((x 1))
      (lambda () x))))

(f)
