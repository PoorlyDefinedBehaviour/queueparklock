use std::{
  cell::UnsafeCell,
  collections::VecDeque,
  ops::{Deref, DerefMut},
  sync::atomic::{AtomicBool, Ordering},
  thread::Thread,
};

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

pub struct Mutex<T: ?Sized> {
  /// Atomic used to acquire a lock before modifying the queue or acquiring the lock.
  guard: AtomicBool,
  /// True when the mutex is acquired by some thread.
  acquired: UnsafeCell<bool>,
  /// FIFO queue of threads waiting to acquire the lock.
  queue: UnsafeCell<VecDeque<Thread>>,
  /// The value being guarded by the mutex.
  value: UnsafeCell<T>,
}

impl<T> Mutex<T> {
  pub fn new(value: T) -> Self {
    Self {
      guard: AtomicBool::new(false),
      acquired: UnsafeCell::new(false),
      queue: UnsafeCell::new(VecDeque::new()),
      value: UnsafeCell::new(value),
    }
  }
}

impl<T: ?Sized> Mutex<T> {
  pub fn lock(&'_ self) -> MutexGuard<'_, T> {
    while self
      .guard
      .compare_exchange_weak(false, true, Ordering::Relaxed, Ordering::Relaxed)
      .is_err()
    {
      // spin loop
    }

    // If we got the mutex, just return a guard and enter the critical session.
    if !unsafe { *self.acquired.get() } {
      unsafe {
        *self.acquired.get() = true;
      }

      self.guard.store(false, Ordering::Release);

      return MutexGuard { mutex: self };
    }

    // Add the current thread to the queue.
    let queue = unsafe { &mut *self.queue.get() };

    queue.push_back(std::thread::current());

    // Release the queue lock.
    self.guard.store(false, Ordering::Release);

    // Sleep until the value lock is released and then try to get the value lock again.
    // (this thread should be able to get the value lock after it is unparked
    // since it will be the only thread that's unparked)
    std::thread::park();

    MutexGuard { mutex: self }
  }

  fn unlock(&self) {
    while self
      .guard
      .compare_exchange_weak(false, true, Ordering::Relaxed, Ordering::Relaxed)
      .is_err()
    {
      // spin loop
    }

    let queue = unsafe { &mut *self.queue.get() };

    match queue.pop_front() {
      // There isn't a thread waiting to acquire the lock.
      None => {
        unsafe { *self.acquired.get() = false };
      }
      // There is some thread waiting to acquire the lock.
      Some(thread) => {
        thread.unpark();
      }
    }

    self.guard.store(false, Ordering::Release);
  }
}

pub struct MutexGuard<'a, T: ?Sized> {
  mutex: &'a Mutex<T>,
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
  fn drop(&mut self) {
    self.mutex.unlock();
  }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { &*self.mutex.value.get() }
  }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *self.mutex.value.get() }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;

  #[test]
  fn smoke() {
    let mutex = Arc::new(Mutex::new(0));

    let mut handles = Vec::new();

    for _ in 0..5 {
      for _ in 0..1000 {
        let mutex = Arc::clone(&mutex);

        handles.push(std::thread::spawn(move || {
          let mut guard = mutex.lock();

          *guard += 1;
        }));
      }
    }

    for handle in handles.into_iter() {
      handle.join().expect("error joining thread");
    }

    assert_eq!(5000, *mutex.lock());
  }
}
