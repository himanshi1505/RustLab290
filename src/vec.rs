// vec_wrapper.rs
#[derive(Debug)]
pub struct VecWrapper<T> {
    storage: Storage<T>,
    len: usize,
}

const STACK_CAPACITY: usize = 8;

#[derive(Debug)]
enum Storage<T> {
    Stack([Option<T>; STACK_CAPACITY]),
    Heap(Vec<T>),
}

impl<T> VecWrapper<T> {
    pub fn new() -> Self {
        Self {
            storage: Storage::Stack(Default::default()),
            len: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        match &mut self.storage {
            Storage::Stack(arr) if self.len < STACK_CAPACITY => {
                arr[self.len] = Some(item);
                self.len += 1;
            }
            Storage::Stack(arr) => {
                let mut vec = Vec::with_capacity(STACK_CAPACITY * 2);
                for item in arr.iter_mut().take(self.len) {
                    vec.push(item.take().unwrap());
                }
                vec.push(item);
                self.storage = Storage::Heap(vec);
                self.len += 1;
            }
            Storage::Heap(vec) => {
                vec.push(item);
                self.len += 1;
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        match &mut self.storage {
            Storage::Stack(arr) => arr[self.len].take(),
            Storage::Heap(vec) => vec.pop(),
        }
    }
}