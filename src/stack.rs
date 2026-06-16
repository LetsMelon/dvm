pub struct Stack {
    inner: Vec<i64>,
}

impl Stack {
    pub fn new(size: usize) -> Stack {
        Stack {
            inner: Vec::with_capacity(size),
        }
    }

    pub fn pop(&mut self) -> Result<i64, String> {
        self.inner.pop().ok_or("Stack underflow".to_string())
    }

    pub fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn push(&mut self, value: i64) {
        self.inner.push(value);
    }

    pub fn get(&self, idx: usize) -> Result<i64, String> {
        self.inner
            .get(idx)
            .cloned()
            .ok_or("Stack Overflow".to_string())
    }

    pub fn top(&self) -> Result<i64, String> {
        self.get(self.size() - 1)
    }

    pub(crate) fn rotate_left_once_last_n(&mut self, n: usize) -> Result<(), String> {
        if n < 2 {
            return Err("Rotate requires n >= 2".to_string());
        }

        let len = self.size();
        if n > len {
            return Err("Stack underflow".to_string());
        }

        let start = len - n;
        let first = self.inner[start];
        for i in start..(len - 1) {
            self.inner[i] = self.inner[i + 1];
        }
        self.inner[len - 1] = first;

        Ok(())
    }
}
