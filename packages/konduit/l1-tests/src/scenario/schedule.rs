use std::collections::BTreeMap;

use crate::scenario::cheque::Stub;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub tick: usize,
    pub kind: Kind,
}

/// Event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Lock { amount: u64, timeout: usize },
    Unlock,
    Squash,
}

/// Map a stub to events
pub fn as_events(stub: &Stub) -> Vec<Event> {
    let timeout = match stub.unlock {
        Some(unlock) => stub.lock + unlock + stub.timeout,
        None => stub.lock + stub.timeout,
    };

    let mut evs = vec![Event {
        tick: stub.lock,
        kind: Kind::Lock {
            amount: stub.amount,
            timeout,
        },
    }];

    if let Some(unlock) = stub.unlock {
        let u = stub.lock + unlock;
        evs.push(Event {
            tick: u,
            kind: Kind::Unlock,
        });
        if let Some(squash) = stub.squash {
            evs.push(Event {
                tick: u + squash,
                kind: Kind::Squash,
            });
        }
    }

    evs
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdxEvent {
    pub idx: u64,
    pub kind: Kind,
}

#[derive(Debug, Clone, Default)]
pub struct Schedule {
    ticks: BTreeMap<usize, Vec<IdxEvent>>,
}

impl Schedule {
    pub fn new(stubs: &[Stub]) -> Self {
        let mut ticks: BTreeMap<usize, Vec<IdxEvent>> = BTreeMap::new();

        for (idx, stub) in stubs.iter().enumerate() {
            for Event { tick, kind } in as_events(stub) {
                let idx = (idx as u64) + 1;
                ticks.entry(tick).or_default().push(IdxEvent { idx, kind });
            }
        }

        Self { ticks }
    }

    pub fn get(&self, tick: &usize) -> Option<&Vec<IdxEvent>> {
        self.ticks.get(tick)
    }
}

#[cfg(test)]
mod scenario_tests {
    use super::*;

    // --- scenarios ---

    #[test]
    fn scenario_locked_only() {
        // amount=100, lock@10, timeout 5 ticks after lock -> times out @15
        let c = Stub::locked(100, 10, 5);
        let evs = as_events(&c);
        assert_eq!(
            evs,
            vec![Event {
                tick: 10,
                kind: Kind::Lock {
                    amount: 100,
                    timeout: 15
                }
            }],
        );
    }

    #[test]
    fn scenario_unlocked_no_squash() {
        // lock@10, unlock 3 ticks after lock -> unlock@13, timeout 5 ticks after unlock -> @18
        let c = Stub::unlocked(100, 10, 3, 5);
        let evs = as_events(&c);
        assert_eq!(
            evs,
            vec![
                Event {
                    tick: 10,
                    kind: Kind::Lock {
                        amount: 100,
                        timeout: 18
                    }
                },
                Event {
                    tick: 13,
                    kind: Kind::Unlock
                },
            ],
        );
    }

    #[test]
    fn scenario_squashed() {
        // lock@10, unlock@13, timeout@18, squash 2 ticks after unlock -> @15
        let c = Stub::squashed(100, 10, 3, 5, 2);
        let evs = as_events(&c);
        assert_eq!(
            evs,
            vec![
                Event {
                    tick: 10,
                    kind: Kind::Lock {
                        amount: 100,
                        timeout: 18
                    }
                },
                Event {
                    tick: 13,
                    kind: Kind::Unlock
                },
                Event {
                    tick: 15,
                    kind: Kind::Squash
                },
            ],
        );
    }

    #[test]
    fn scenario_lock_at_zero() {
        let c = Stub::locked(50, 0, 7);
        let evs = as_events(&c);
        assert_eq!(
            evs,
            vec![Event {
                tick: 0,
                kind: Kind::Lock {
                    amount: 50,
                    timeout: 7
                }
            }]
        );
    }

    #[test]
    fn scenario_zero_length_unlock_gap() {
        // unlock == 0: Unlock lands on the same tick as Lock
        let c = Stub::unlocked(100, 10, 0, 5);
        let evs = as_events(&c);
        assert_eq!(
            evs,
            vec![
                Event {
                    tick: 10,
                    kind: Kind::Lock {
                        amount: 100,
                        timeout: 15
                    }
                },
                Event {
                    tick: 10,
                    kind: Kind::Unlock
                },
            ],
        );
    }

    #[test]
    fn scenario_zero_length_squash_gap() {
        // squash == 0: Squash lands on the same tick as Unlock
        let c = Stub::squashed(100, 10, 3, 5, 0);
        let evs = as_events(&c);
        assert_eq!(
            evs,
            vec![
                Event {
                    tick: 10,
                    kind: Kind::Lock {
                        amount: 100,
                        timeout: 18
                    }
                },
                Event {
                    tick: 13,
                    kind: Kind::Unlock
                },
                Event {
                    tick: 13,
                    kind: Kind::Squash
                },
            ],
        );
    }

    // --- Schedule::new scenarios ---

    #[test]
    fn scenario_schedule_single_cheque() {
        let stubs = vec![Stub::locked(100, 0, 5)];
        let schedule = Schedule::new(&stubs);
        assert_eq!(schedule.ticks.len(), 1);
        assert_eq!(
            schedule.ticks[&0],
            vec![IdxEvent {
                idx: 1,
                kind: Kind::Lock {
                    amount: 100,
                    timeout: 5
                }
            }],
        );
    }

    #[test]
    fn scenario_schedule_multiple_stubs_distinct_ticks() {
        let stubs = vec![
            Stub::locked(100, 0, 5),  // lock@0
            Stub::locked(200, 10, 5), // lock@10
        ];
        let schedule = Schedule::new(&stubs);
        assert_eq!(
            schedule.ticks.keys().copied().collect::<Vec<_>>(),
            vec![0, 10]
        );
        assert_eq!(
            schedule.ticks[&0],
            vec![IdxEvent {
                idx: 1,
                kind: Kind::Lock {
                    amount: 100,
                    timeout: 5
                }
            }],
        );
        assert_eq!(
            schedule.ticks[&10],
            vec![IdxEvent {
                idx: 2,
                kind: Kind::Lock {
                    amount: 200,
                    timeout: 15
                }
            }],
        );
    }

    #[test]
    fn scenario_schedule_collision_same_tick() {
        let stubs = vec![
            Stub::unlocked(100, 0, 5, 3), // unlock@5
            Stub::locked(200, 5, 10),     // lock@5
        ];
        let schedule = Schedule::new(&stubs);
        let at_5 = &schedule.ticks[&5];
        assert_eq!(at_5.len(), 2);
        assert_eq!(
            at_5[0],
            IdxEvent {
                idx: 1,
                kind: Kind::Unlock
            }
        );
        assert_eq!(
            at_5[1],
            IdxEvent {
                idx: 2,
                kind: Kind::Lock {
                    amount: 200,
                    timeout: 15
                }
            }
        );
    }

    #[test]
    fn scenario_schedule_empty_input() {
        let schedule = Schedule::new(&[]);
        assert!(schedule.ticks.is_empty());
    }

    #[test]
    fn scenario_schedule_preserves_idx_across_gaps() {
        let stubs = vec![
            Stub::squashed(100, 0, 5, 3, 1), // 3 events: lock@0, unlock@5, squash@6
            Stub::locked(200, 1, 2),         // 1 event: lock@1
        ];
        let schedule = Schedule::new(&stubs);
        assert_eq!(schedule.ticks[&0][0].idx, 1);
        assert_eq!(schedule.ticks[&1][0].idx, 2);
        assert_eq!(schedule.ticks[&5][0].idx, 1);
        assert_eq!(schedule.ticks[&6][0].idx, 1);
    }
}
