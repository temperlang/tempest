//! The settle loop's scheduler, apart from what its steps do: work items
//! that each wait on at most one fact, an index from facts to their
//! waiters, and a ready queue in a fixed order.
//!
//! - Items run in order of (phase, source position, creation), so a build's
//!   output and errors don't depend on hash order or timing.
//! - A step that can't go on waits on one fact ([`Engine::wait`]); publishing
//!   that fact ([`Engine::publish`]) makes exactly its waiters ready again.
//!   Facts are only ever added.
//! - When nothing is ready but items still wait, the earliest item whose
//!   wait can't end is forced ([`Next::Force`]): one waiting on a fact that
//!   nothing will produce, or on a cycle of waits. Its step must go on
//!   without the fact, usually by making an error node, which may let
//!   others go on. Forcing the earliest one first keeps follow-on errors
//!   down and the result deterministic.
//! - A step budget stops runaway work ([`Next::OutOfBudget`]).
//!
//! [`Engine::chain`] answers "why is this waiting?", for errors and
//! debugging: the facts and items an item's wait leads through.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::Hash;

/// A work item's id, in order of creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub u32);

/// What to do next.
#[derive(Debug, PartialEq, Eq)]
pub enum Next<F> {
    /// Run this item's step.
    Run(ItemId),
    /// Run this item's step knowing that the fact it waits on won't come.
    Force(ItemId, F),
    /// Nothing is left to do.
    Settled,
    /// The step budget ran out, with this item next.
    OutOfBudget(ItemId),
}

#[derive(Clone, Copy, Debug)]
struct Item<F> {
    phase: u8,
    pos: u32,
    waiting: Option<F>,
    done: bool,
    /// Whether it's in the ready queue.
    queued: bool,
}

/// The scheduler, for facts of type `F`.
#[derive(Debug)]
pub struct Engine<F> {
    items: Vec<Item<F>>,
    ready: BinaryHeap<Reverse<(u8, u32, ItemId)>>,
    waiters: HashMap<F, Vec<ItemId>>,
    producers: HashMap<F, ItemId>,
    published: HashSet<F>,
    steps: u64,
    budget: u64,
}

impl<F: Copy + Eq + Hash> Engine<F> {
    pub fn new(budget: u64) -> Engine<F> {
        Engine {
            items: Vec::new(),
            ready: BinaryHeap::new(),
            waiters: HashMap::new(),
            producers: HashMap::new(),
            published: HashSet::new(),
            steps: 0,
            budget,
        }
    }

    /// Adds a ready item. Lower phases run first, then lower positions.
    pub fn add(&mut self, phase: u8, pos: u32) -> ItemId {
        let id = ItemId(self.items.len() as u32);
        self.items.push(Item {
            phase,
            pos,
            waiting: None,
            done: false,
            queued: false,
        });
        self.enqueue(id);
        id
    }

    fn enqueue(&mut self, id: ItemId) {
        let item = &mut self.items[id.0 as usize];
        if !item.queued && !item.done {
            item.queued = true;
            self.ready.push(Reverse((item.phase, item.pos, id)));
        }
    }

    /// Allows `extra` more steps, as for each round of a session that
    /// grows ([`crate::settle::Incremental`]).
    pub fn grow_budget(&mut self, extra: u64) {
        self.budget = self.budget.saturating_add(extra);
    }

    /// Drops every item not done, as if done, after a round that
    /// stopped partway; facts they'd have produced are left unpublished.
    pub fn abandon(&mut self) {
        for item in &mut self.items {
            item.done = true;
            item.queued = false;
            item.waiting = None;
        }
        self.ready.clear();
        self.waiters.clear();
        self.producers.clear();
    }

    /// Steps run so far.
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// The next item to run, if any.
    pub fn next_step(&mut self) -> Next<F> {
        if let Some(Reverse((_, _, id))) = self.ready.pop() {
            self.items[id.0 as usize].queued = false;
            if self.steps >= self.budget {
                return Next::OutOfBudget(id);
            }
            self.steps += 1;
            return Next::Run(id);
        }
        match self.stuck() {
            Some((id, fact)) => {
                if self.steps >= self.budget {
                    return Next::OutOfBudget(id);
                }
                self.steps += 1;
                self.unwait(id);
                Next::Force(id, fact)
            }
            None => Next::Settled,
        }
    }

    /// The earliest waiting item whose wait can't end: its chain of waits
    /// reaches a fact nobody will produce, or loops.
    fn stuck(&self) -> Option<(ItemId, F)> {
        let mut best: Option<(u8, u32, ItemId, F)> = None;
        for (i, item) in self.items.iter().enumerate() {
            let (Some(fact), false) = (item.waiting, item.done) else {
                continue;
            };
            let id = ItemId(i as u32);
            if !self.can_end(id)
                && best
                    .is_none_or(|(phase, pos, b, _)| (item.phase, item.pos, id) < (phase, pos, b))
            {
                best = Some((item.phase, item.pos, id, fact));
            }
        }
        best.map(|(_, _, id, fact)| (id, fact))
    }

    /// Whether an item's chain of waits ends at a producer that can still
    /// run: one that is ready.
    fn can_end(&self, id: ItemId) -> bool {
        let mut seen = HashSet::new();
        let mut at = id;
        loop {
            if !seen.insert(at) {
                return false;
            }
            let item = &self.items[at.0 as usize];
            if item.done {
                return false;
            }
            let Some(fact) = item.waiting else {
                return true;
            };
            match self.producers.get(&fact) {
                Some(&producer) => at = producer,
                None => return false,
            }
        }
    }

    /// Makes an item wait on a fact, or ready again if it's known already.
    pub fn wait(&mut self, id: ItemId, fact: F) {
        if self.published.contains(&fact) {
            self.enqueue(id);
            return;
        }
        self.items[id.0 as usize].waiting = Some(fact);
        self.waiters.entry(fact).or_default().push(id);
    }

    fn unwait(&mut self, id: ItemId) {
        if let Some(fact) = self.items[id.0 as usize].waiting.take() {
            if let Some(waiters) = self.waiters.get_mut(&fact) {
                waiters.retain(|&w| w != id);
            }
        }
    }

    /// Marks an item finished.
    pub fn done(&mut self, id: ItemId) {
        self.unwait(id);
        self.items[id.0 as usize].done = true;
    }

    /// Makes an item ready to run again, as after a step that did part of
    /// its work.
    pub fn again(&mut self, id: ItemId) {
        self.unwait(id);
        self.enqueue(id);
    }

    /// Records that an item will publish a fact, for finding stuck items and
    /// explaining waits.
    pub fn will_produce(&mut self, fact: F, id: ItemId) {
        self.producers.insert(fact, id);
    }

    /// Adds a fact, making its waiters ready.
    pub fn publish(&mut self, fact: F) {
        if !self.published.insert(fact) {
            return;
        }
        self.producers.remove(&fact);
        for id in self.waiters.remove(&fact).unwrap_or_default() {
            self.items[id.0 as usize].waiting = None;
            self.enqueue(id);
        }
    }

    pub fn is_published(&self, fact: F) -> bool {
        self.published.contains(&fact)
    }

    /// Why an item waits: each fact in its chain of waits, with the item
    /// that would produce it, if any. Stops at a repeat.
    pub fn chain(&self, id: ItemId) -> Vec<(F, Option<ItemId>)> {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut at = id;
        while seen.insert(at) {
            let Some(fact) = self.items[at.0 as usize].waiting else {
                break;
            };
            let producer = self.producers.get(&fact).copied();
            chain.push((fact, producer));
            match producer {
                Some(p) => at = p,
                None => break,
            }
        }
        chain
    }

    /// The fact an item waits on.
    pub fn waiting(&self, id: ItemId) -> Option<F> {
        self.items[id.0 as usize].waiting
    }

    /// Whether any item is unfinished.
    pub fn is_busy(&self) -> bool {
        self.items.iter().any(|item| !item.done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs items that each wait on facts in turn, as listed, publishing
    /// their own fact when done; forced items give up their wait. Returns
    /// the order of events.
    fn simulate(plan: &[(u8, u32, &[u32], Option<u32>)], budget: u64) -> Vec<String> {
        let mut engine = Engine::new(budget);
        let mut progress = vec![0; plan.len()];
        for (i, &(phase, pos, _, produces)) in plan.iter().enumerate() {
            let id = engine.add(phase, pos);
            if let Some(fact) = produces {
                engine.will_produce(fact, id);
            }
            assert_eq!(id, ItemId(i as u32));
        }
        let mut log = Vec::new();
        loop {
            let (id, forced) = match engine.next_step() {
                Next::Run(id) => (id, false),
                Next::Force(id, fact) => {
                    log.push(format!("force {} on {fact}", id.0));
                    (id, true)
                }
                Next::Settled => break,
                Next::OutOfBudget(id) => {
                    log.push(format!("budget at {}", id.0));
                    break;
                }
            };
            let i = id.0 as usize;
            let (_, _, waits, produces) = plan[i];
            if forced {
                progress[i] += 1;
            }
            if let Some(&fact) = waits.get(progress[i]) {
                if !engine.is_published(fact) {
                    log.push(format!("{i} waits on {fact}"));
                    engine.wait(id, fact);
                    continue;
                }
                progress[i] += 1;
                engine.again(id);
                continue;
            }
            log.push(format!("{i} done"));
            engine.done(id);
            if let Some(fact) = produces {
                engine.publish(fact);
            }
        }
        log
    }

    #[test]
    fn runs_in_phase_and_position_order() {
        let log = simulate(
            &[(1, 0, &[], None), (0, 9, &[], None), (0, 3, &[], None)],
            100,
        );
        assert_eq!(log, ["2 done", "1 done", "0 done"]);
    }

    #[test]
    fn publishing_wakes_exactly_the_waiters() {
        let log = simulate(
            &[(0, 0, &[7], None), (0, 1, &[8], None), (0, 2, &[], Some(7))],
            100,
        );
        assert_eq!(
            log,
            [
                "0 waits on 7",
                "1 waits on 8",
                "2 done",
                "0 done",
                "force 1 on 8",
                "1 done"
            ]
        );
    }

    #[test]
    fn cycles_break_at_the_earliest_item() {
        // 0 waits on 1's fact, 1 on 2's, 2 on 0's.
        let plan: &[(u8, u32, &[u32], Option<u32>)] = &[
            (0, 5, &[11], Some(10)),
            (0, 1, &[12], Some(11)),
            (0, 3, &[10], Some(12)),
        ];
        let log = simulate(plan, 100);
        assert_eq!(
            log,
            [
                "1 waits on 12",
                "2 waits on 10",
                "0 waits on 11",
                "force 1 on 12",
                "1 done",
                "0 done",
                "2 done"
            ]
        );
    }

    #[test]
    fn chains_explain_waits() {
        let mut engine: Engine<u32> = Engine::new(100);
        let a = engine.add(0, 0);
        let b = engine.add(0, 1);
        engine.will_produce(1, b);
        engine.wait(a, 1);
        engine.wait(b, 2);
        assert_eq!(engine.chain(a), [(1, Some(b)), (2, None)]);
    }

    #[test]
    fn budget_stops_work() {
        let log = simulate(&[(0, 0, &[], None), (0, 1, &[], None)], 1);
        assert_eq!(log, ["0 done", "budget at 1"]);
    }
}
