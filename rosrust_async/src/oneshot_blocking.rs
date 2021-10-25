#![allow(dead_code)]

use std::sync::{ Arc, Mutex, Condvar };

#[derive(Debug)]
pub enum TryRecvError {
    Empty,
    Closed,
}

#[derive(Debug)]
pub struct RecvError;

pub struct Receiver<T> {
    cond_pair: Arc<(Mutex<State<T>>, Condvar)>,
}

impl<T> Receiver<T> {
    pub fn recv(self) -> Result<T, RecvError> {
        let (mutex, condvar) = &*self.cond_pair;
        let mut state = mutex.lock().unwrap();

        loop {
            match state.take() {
                State::Sent(val) => break Ok(val),
                State::Closed => break Err(RecvError),
                State::NotSent => {
                    state = condvar.wait(state).unwrap();
                },
            }
        }
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        let (mutex, _) = &*self.cond_pair;
        let mut state = mutex.lock().unwrap();

        match state.take() {
            State::Sent(val) => Ok(val),
            State::NotSent      => Err(TryRecvError::Empty),
            State::Closed       => Err(TryRecvError::Closed),
        }
    }
}

pub struct Sender<T> {
    cond_pair: Arc<(Mutex<State<T>>, Condvar)>,
}

impl<T> Sender<T> {
    pub fn send(self, val: T) -> Result<(), T> {
        let (mutex, condvar) = &*self.cond_pair;
        let mut state = mutex.lock().unwrap();

        if let State::Closed = &*state {
            Err(val)
        } else {
            *state = State::Sent(val);
            condvar.notify_one();
            Ok(())
        }
    }
}

enum State<T> {
    Sent(T),
    NotSent,
    Closed,
}

impl<T> State<T> {
    fn take(&mut self) -> State<T> {
        std::mem::replace(self, State::NotSent)
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let (mutex, condvar) = &*self.cond_pair;
        let mut state = mutex.lock().unwrap();

        if let State::NotSent = &*state {
            *state = State::Closed;
            condvar.notify_one();
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let (mutex, condvar) = &*self.cond_pair;
        let mut state = mutex.lock().unwrap();

        if let State::NotSent = &*state {
            *state = State::Closed;
            condvar.notify_one();
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let cond_pair = Arc::new((Mutex::new(State::NotSent), Condvar::new()));
    let tx = Sender { cond_pair: Arc::clone(&cond_pair) };
    let rx = Receiver { cond_pair };
    (tx, rx)
}
