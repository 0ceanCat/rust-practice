struct Node<T> {
    data: T,
    next: Option<Box<Node<T>>>,
}

pub(crate) struct List<T> {
    head: Option<Box<Node<T>>>,
    pub(crate) size: u32
}

pub struct IntoIter<T>(List<T>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

pub struct Iter<'a, T> {
    next: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.next.as_deref();
            &node.data
        })
    }
}

pub struct IterMut<'a, T> {
    next: Option<&'a mut Node<T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_deref_mut();
            &mut node.data
        })
    }
}


impl<T> List<T> {
    pub(crate) fn new() -> Self {
        List {
            head: None,
            size: 0
        }
    }

    pub(crate) fn push(&mut self, data: T) {
        self.head = Some(Box::new(Node { data, next: self.head.take()}));
        self.size += 1;
    }

    pub(crate) fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|x| &x.data)
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        self.head.take().map(|x| {
            self.head = x.next;
            self.size  -= 1;
            x.data
        })
    }

    pub(crate) fn remove(&mut self, index: u32) -> Option<T> {
        if index >= self.size {
            return None;
        }

        if index == 0 {
            return self.head.take().map(|x| {
                self.head = x.next;
                self.size -= 1;
                x.data
            })
        }

        let mut current_node = self.head.as_deref_mut();
        let mut current_index = 0;

        while current_index < index - 1 {
            if let Some(node) = current_node{
                current_node = node.next.as_deref_mut();
            }
            current_index += 1;
        }

        current_node.take().map(|x| {
            return x.next.take().map(|next| {
                let result = x.next.take().map(|x| x.data);
                x.next = next.next;
                self.size -= 1;
                result
            }).unwrap_or(None);
        }).unwrap_or(None)
     }

    pub fn into_iter(self) -> IntoIter<T> {
        IntoIter(self)
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter { next: self.head.as_deref() }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { next: self.head.as_deref_mut() }
    }
 }