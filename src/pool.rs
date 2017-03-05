extern crate std;

use std::ops::{Index,IndexMut};

pub enum Slot<T> {
    Free(usize),                 // Index of next entry in freelist, -1 for none
    Alloc(T),
}

/// Simple fixed size pool allocator.
pub struct Pool<T> {
    pub pool: Vec<Slot<T>>,
    freelist: isize,
    used: usize,
    next: usize
}

impl<T> Pool<T> {
    /// Create a new pool with a given size.
    pub fn new(size: usize) -> Pool<T> {
        assert!(size > 0);
        Pool { pool: (1 .. size + 1).map(Slot::Free).collect(),
               freelist: (size - 1) as isize,
               used: 0,
               next: 0 }
    }

    /// Allocate an index in the pool. Returns None if the Pool is all used.
    pub fn allocidx(&mut self, init: T) -> Result<usize, T> {
        let idx = self.next;
        if idx >= self.pool.len() {
            Err(init)
        } else {
            self.next = match self.pool[idx] {
                Slot::Free(next) => next,
                Slot::Alloc(_) => panic!("Pool slot already contains a value")
            };
            self.pool[idx] = Slot::Alloc(init);
            self.used += 1;
            Ok(idx)
        }
    }

    /// Free an index in the pool
    pub fn freeidx(&mut self, idx: usize) -> T {
        assert!(idx < self.pool.len());
        let next = self.next;
        self.used -= 1;
        match std::mem::replace(&mut self.pool[idx], Slot::Free(next)) {
            Slot::Alloc(v) => {
                self.next = idx;
                v
            },
            Slot::Free(_) => panic!("Freeing free entry {}", idx)
        }
    }

    /// Allow an entry to be freed from a raw pointer. Inherently unsafe.
    pub unsafe fn freeptr(&mut self, ptr: *const T) -> T {
        assert!(ptr as usize >= self.pool.as_ptr() as usize);
        // divide rounds down so it doesn't matter if its in the middle of Slot<>
        let idx = ((ptr as usize) - (self.pool.as_ptr() as usize)) / std::mem::size_of::<Slot<T>>();
        self.freeidx(idx)
    }

    /// Return the max number of pool entries (size passed to new()).
    #[allow(dead_code)]
    pub fn limit(&self) -> usize { self.pool.len() }

    /// Return number of currently allocated entries.
    #[allow(dead_code)]
    pub fn used(&self) -> usize { self.used }

    /// Return number of remaining unused entries.
    #[allow(dead_code)]
    pub fn avail(&self) -> usize { self.limit() - self.used() }
}

impl<T> Index<usize> for Pool<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        match self.pool[idx] {
            Slot::Free(_) => panic!("access free index {}", idx),
            Slot::Alloc(ref t) => t
        }
    }
}

impl<T> IndexMut<usize> for Pool<T> {
    fn index_mut(&mut self, idx: usize) -> &mut T {
        match &mut self.pool[idx] {
            &mut Slot::Free(_) => panic!("access free index {}", idx),
            &mut Slot::Alloc(ref mut t) => t
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Pool, Slot};

    fn print(p: &Pool<i32>) {
        for (i, slot) in p.pool.iter().enumerate() {
            match slot {
                &Slot::Free(freelist) => println!("slot {} free {}", i, freelist),
                &Slot::Alloc(v) => println!("slot {} value {:?}", i, v)
            }
        }
    }
    
    #[test]
    fn alloc() {
        let mut p = Pool::new(4);

        assert!(p.limit() == 4);
        assert!(p.used() == 0);
        assert!(p.avail() == 4);

        for i in 0..4 {
            let idx = p.allocidx(i);

            assert!(p.used() == (i + 1) as usize);
            assert!(idx.is_ok());
            assert!(p[idx.ok().unwrap()] == i);
        }

        assert!(p.avail() == 0);
        let idx = p.allocidx(10);
        assert!(p.avail() == 0);
        assert!(idx.is_err());
    }


    #[test]
    fn reuse_slot() {
        let mut p = Pool::new(3);

        assert_eq!(Ok(0), p.allocidx(1));
        assert_eq!(1, p.freeidx(0));
        assert_eq!(Ok(0), p.allocidx(2));
        assert_eq!(2, p.freeidx(0));
    }


    #[test]
    fn free() {
        let mut p = Pool::new(4);
        let mut v = Vec::new();

        assert!(p.limit() == 4);
        assert!(p.used() == 0);
        assert!(p.avail() == 4);

        for i in 0..20 {
            let idx = p.allocidx(i);

            assert!(idx.is_ok());
            assert!(idx.unwrap() < 4);
            assert!(p[idx.unwrap()] == i);

            v.push(idx.unwrap());

            if p.avail() == 0 {
                p.freeidx(v.remove(0));
                assert!(p.avail() == 1);
            }
        }
    }

    #[test]
    fn freeptr() {
        let mut p = Pool::new(4);
        let mut v = Vec::new();

        assert!(p.limit() == 4);
        assert!(p.used() == 0);
        assert!(p.avail() == 4);

        for i in 0..20 {
            let idx = p.allocidx(i);

            assert!(idx.is_ok());
            assert!(idx.ok().unwrap() < 4);
            assert!(p[idx.ok().unwrap()] == i);

            v.push(&p[idx.ok().unwrap()] as *const isize);

            if p.avail() == 0 {
                unsafe { p.freeptr(v.remove(0)) };
                assert!(p.avail() == 1);
            }
        }
    }

    #[test]
    #[should_panic]
    fn badfree1() {
        let mut p = Pool::new(4);

        let idx = p.allocidx(0);
        assert!(idx.is_ok());

        p.freeidx(idx.ok().unwrap() + 1);
    }

    #[test]
    #[should_panic]
    fn badfree2() {
        let mut p = Pool::new(4);

        let idx = p.allocidx(0);
        assert!(idx.is_ok());

        p.freeidx(idx.ok().unwrap() - 1);
    }

    #[test]
    #[should_panic]
    fn badidx0() {
        let mut p = Pool::new(4);

        p[0] = 1;
    }

    #[test]
    #[should_panic]
    fn badidx1() {
        let mut p = Pool::new(4);

        let idx = p.allocidx(0);
        assert!(idx.is_ok());

        p[idx.ok().unwrap() + 1] = 1;
    }

    #[test]
    #[should_panic]
    fn badidx2() {
        let mut p = Pool::new(4);

        let idx = p.allocidx(0);
        assert!(idx.is_ok());

        p[idx.ok().unwrap() - 1] = 1;
    }

    #[test]
    #[should_panic]
    fn badptr1() {
        let mut p = Pool::new(4);
        let foo : isize = 1;

        let idx = p.allocidx(0);
        assert!(idx.is_ok());

        unsafe { p.freeptr(&foo as *const isize) };
    }

    #[test]
    #[should_panic]
    fn badptr2() {
        let mut p = Pool::new(4);

        let idx = p.allocidx(0);
        assert!(idx.is_ok());

        unsafe {
            let ptr = ((&p[0] as *const isize as usize) - 256) as *const isize;
            p.freeptr(ptr)
        };
    }
}
