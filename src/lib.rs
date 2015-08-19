//! An implementation of the [Hybrid Logical Clock](http://www.cse.buffalo.edu/tech-reports/2014-04.pdf) for Rust.

extern crate time;

use std::sync::{Mutex};

#[derive(Debug,Clone,Copy,Eq,PartialEq,PartialOrd,Ord)]
pub struct HLTimespec {
    wall: time::Timespec,
    logical: u16,
}

pub struct State {
    s: Mutex<HLTimespec>,
    now: Box<FnMut() -> time::Timespec>,
}

impl State {
    pub fn new() -> State {
        State{s:Mutex::new(HLTimespec{wall: time::Timespec{sec:0, nsec:0 }, logical: 0}), now: Box::new(time::get_time)}
    }
    pub fn get_time(&mut self) -> HLTimespec {
        let mut s = self.s.lock().unwrap();
        let wall = (self.now)();
        if s.wall < wall {
            s.wall = wall;
            s.logical = 0;
        } else {
            s.logical += 1;
        }
        s.clone()
    }
    pub fn update(&mut self, event: HLTimespec) -> HLTimespec {
        let mut s = self.s.lock().unwrap();
        let wall = (self.now)();

        if wall > event.wall && wall > s.wall {
            s.wall = wall;
            s.logical = 0
        } else if event.wall > s.wall {
            s.wall = event.wall;
            s.logical = event.logical+1;
        } else if s.wall > event.wall {
            s.logical += 1;
        } else {
            if event.logical > s.logical {
                s.logical = event.logical;
            }
            s.logical += 1;
        }
        s.clone()
    }
}

#[cfg(test)]
mod tests {
    extern crate time;
    use {HLTimespec,State};

    fn ts(s: i64, ns: i32) -> time::Timespec {
        time::Timespec{sec: s, nsec: ns}
    }

    fn hlts(s: i64, ns: i32, l: u16) -> HLTimespec {
        HLTimespec{wall: ts(s,ns), logical: l}
    }


    #[test]
    fn it_works() {
        let zero = hlts(0,0,0);
        let ops = vec![
            // Test cases in the form (wall, event_ts, outcome).
            // Specifying event_ts as zero corresponds to calling `get_time`,
            // otherwise `update`.
            (ts(1,0), zero, hlts(1,0,0)),
            (ts(1,0), zero, hlts(1,0,1)), // clock didn't move
            (ts(0,9), zero, hlts(1,0,2)), // clock moved back
            (ts(2,0), zero, hlts(2,0,0)), // finally ahead again
            (ts(3,0), hlts(1,2,3), hlts(3,0,0)), // event happens, but wall ahead
            (ts(3,0), hlts(1,2,3), hlts(3,0,1)), // event happens, wall ahead but unchanged
            (ts(3,0), hlts(3,0,1), hlts(3,0,2)), // event happens at wall, which is still unchanged
            (ts(3,0), hlts(3,0,99), hlts(3,0,100)), // event with larger logical, wall unchanged
            (ts(3,5), hlts(4,4,100), hlts(4,4,101)), // event with larger wall than state, wall behind
            (ts(5,0), hlts(4,5,0), hlts(5,0,0)), // event behind wall, but ahead of previous state
            (ts(4,9), hlts(5,0,99), hlts(5,0,100)),
            (ts(0,0), hlts(5,0,50), hlts(5,0,101)), // event at state, lower logical than state
        ];

        // Prepare fake clock.
        let mut times = ops.iter().rev().map(|op| op.0).collect::<Vec<time::Timespec>>();
        let now: Box<FnMut() -> time::Timespec> = Box::new(move || times.pop().unwrap());
    
        let mut s = State::new();
        s.now = now;

        for op in &ops {
            let t = if op.1 == zero { s.get_time() } else { s.update(op.1.clone()) };
            assert_eq!(t, op.2);
        }
    }
}
