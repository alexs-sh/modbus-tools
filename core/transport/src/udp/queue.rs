use std::collections::VecDeque;

pub struct FixedQueue<T> {
    data: VecDeque<Option<T>>,
    limit: usize,
}

impl<T> FixedQueue<T> {
    pub fn new(limit: usize) -> FixedQueue<T> {
        FixedQueue {
            data: VecDeque::new(),
            limit,
        }
    }

    pub fn push(&mut self, value: T) -> bool {
        if self.data.len() < self.limit {
            self.data.push_back(Some(value));
            true
        } else {
            false
        }
    }

    pub fn count_free(&self) -> usize {
        self.limit - self.len()
    }

    pub fn push_replace(&mut self, value: T) -> bool {
        if self.count_free() > 0 {
            self.push(value)
        } else {
            self.data.pop_front();
            self.push(value)
        }
    }

    pub fn take_if<P>(&mut self, predicate: P) -> Option<T>
    where
        P: Fn(&T) -> bool,
    {
        if let Some(pos) = self
            .data
            .iter()
            .position(|e| e.as_ref().map_or(false, &predicate))
        {
            let value = self.data[pos].take();
            let len = self.data.len();
            if len > 1 {
                self.data.swap(pos, len - 1);
            }
            self.data.pop_back();
            value
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push() {
        let mut storage = FixedQueue::<i32>::new(4);

        for i in 0..10 {
            storage.push(i);
        }

        assert_eq!(storage.len(), 4);
        assert_eq!(storage.count_free(), 0);

        let r = storage.take_if(|x| *x == 10);
        assert_eq!(r, None);

        let r = storage.take_if(|x| *x == 0);
        assert_eq!(r, Some(0));
        assert_eq!(storage.len(), 3);
        assert_eq!(storage.count_free(), 1);

        let r = storage.take_if(|x| *x == 0);
        assert_eq!(r, None);
        assert_eq!(storage.len(), 3);
        assert_eq!(storage.count_free(), 1);

        let r = storage.take_if(|x| *x == 1);
        assert_eq!(r, Some(1));
        assert_eq!(storage.len(), 2);
        assert_eq!(storage.count_free(), 2);
    }

    #[test]
    fn push_replace() {
        let mut storage = FixedQueue::<i32>::new(4);

        for i in 0..10 {
            storage.push_replace(i);
        }

        assert_eq!(storage.len(), 4);

        let r = storage.take_if(|x| *x == 9);
        assert_eq!(r, Some(9));
        assert_eq!(storage.len(), 3);

        let r = storage.take_if(|x| *x == 8);
        assert_eq!(r, Some(8));
        assert_eq!(storage.len(), 2);

        let r = storage.take_if(|x| *x == 7);
        assert_eq!(r, Some(7));
        assert_eq!(storage.len(), 1);

        let r = storage.take_if(|x| *x == 6);
        assert_eq!(r, Some(6));
        assert_eq!(storage.len(), 0);
    }
}
