pub struct Stack {
    inner: Box<[u8]>,
    len: usize,
}

impl Stack {
    pub fn new(capacity: usize) -> Stack {
        Stack {
            inner: vec![0; capacity].into_boxed_slice(),
            len: 0,
        }
    }

    pub fn len_bytes(&self) -> usize {
        self.len
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.ensure_capacity_for(bytes.len())?;

        let end = self.len + bytes.len();
        self.inner[self.len..end].copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }

    pub fn pop_bytes(&mut self, len: usize) -> Result<Vec<u8>, String> {
        if len > self.len {
            return Err("Stack underflow".to_string());
        }

        let start = self.len - len;
        let bytes = self.inner[start..self.len].to_vec();
        self.len = start;
        Ok(bytes)
    }

    pub fn peek_bytes(&self, len: usize) -> Result<Vec<u8>, String> {
        if len > self.len {
            return Err("Stack underflow".to_string());
        }

        Ok(self.inner[self.len - len..self.len].to_vec())
    }

    pub fn dump_bytes(&self) -> Vec<u8> {
        self.inner[..self.len].to_vec()
    }

    pub fn push_i32(&mut self, value: i32) -> Result<(), String> {
        self.push_bytes(&value.to_le_bytes())
    }

    pub fn pop_i32(&mut self) -> Result<i32, String> {
        let bytes = self.pop_bytes(4)?;
        Ok(i32::from_le_bytes(
            bytes.try_into().expect("len already checked"),
        ))
    }

    pub fn peek_i32(&self) -> Result<i32, String> {
        let bytes = self.peek_bytes(4)?;
        Ok(i32::from_le_bytes(
            bytes.try_into().expect("len already checked"),
        ))
    }

    pub fn push_u8(&mut self, value: u8) -> Result<(), String> {
        self.ensure_capacity_for(1)?;
        self.inner[self.len] = value;
        self.len += 1;
        Ok(())
    }

    pub fn pop_u8(&mut self) -> Result<u8, String> {
        if self.len == 0 {
            return Err("Stack underflow".to_string());
        }

        self.len -= 1;
        Ok(self.inner[self.len])
    }

    pub fn peek_u8(&self) -> Result<u8, String> {
        if self.len == 0 {
            return Err("Stack underflow".to_string());
        }

        Ok(self.inner[self.len - 1])
    }

    pub(crate) fn len_i32s(&self) -> Result<usize, String> {
        if self.len % 4 != 0 {
            return Err("Stack is not aligned to i32 values".to_string());
        }

        Ok(self.len / 4)
    }

    pub(crate) fn get_i32(&self, idx: usize) -> Result<i32, String> {
        let start = idx
            .checked_mul(4)
            .ok_or_else(|| "Stack index overflow".to_string())?;
        let end = start + 4;

        let bytes = self
            .inner
            .get(start..end)
            .filter(|_| end <= self.len)
            .ok_or("Stack underflow".to_string())?;

        Ok(i32::from_le_bytes(
            bytes.try_into().expect("slice length is 4"),
        ))
    }

    pub(crate) fn rotate_left_once_last_n_i32(&mut self, n: usize) -> Result<(), String> {
        if n < 2 {
            return Err("Swap requires n >= 2".to_string());
        }

        let len = self.len_i32s()?;
        if n > len {
            return Err("Stack underflow".to_string());
        }

        let start = (len - n) * 4;
        self.inner[start..self.len].rotate_left(4);
        Ok(())
    }

    fn ensure_capacity_for(&self, additional: usize) -> Result<(), String> {
        let new_len = self
            .len
            .checked_add(additional)
            .ok_or("Stack capacity overflow".to_string())?;

        if new_len > self.inner.len() {
            return Err(format!(
                "Stack overflow: capacity is {} bytes, attempted to grow to {} bytes",
                self.inner.len(),
                new_len
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Stack;

    #[test]
    fn i32_round_trip_preserves_value() {
        let mut stack = Stack::new(16);

        stack.push_i32(-123456).unwrap();

        assert_eq!(stack.peek_i32().unwrap(), -123456);
        assert_eq!(stack.pop_i32().unwrap(), -123456);
        assert_eq!(stack.len_bytes(), 0);
    }

    #[test]
    fn byte_round_trip_preserves_order() {
        let mut stack = Stack::new(8);

        stack.push_bytes(&[1, 2, 3, 4]).unwrap();

        assert_eq!(stack.peek_bytes(3).unwrap(), vec![2, 3, 4]);
        assert_eq!(stack.pop_bytes(4).unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn push_u8_and_dump_bytes_share_the_same_stack_surface() {
        let mut stack = Stack::new(4);

        stack.push_u8(65).unwrap();
        stack.push_u8(66).unwrap();

        assert_eq!(stack.dump_bytes(), vec![65, 66]);
        assert_eq!(stack.pop_u8().unwrap(), 66);
        assert_eq!(stack.peek_u8().unwrap(), 65);
    }

    #[test]
    fn rejects_push_that_exceeds_fixed_capacity() {
        let mut stack = Stack::new(4);

        assert!(stack.push_bytes(&[1, 2, 3, 4]).is_ok());
        let error = stack.push_u8(5).unwrap_err();

        assert!(error.contains("Stack overflow"));
    }
}
