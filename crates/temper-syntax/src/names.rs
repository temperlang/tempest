//! Interned names: each distinct name gets a small id, so comparing names is
//! comparing integers. Names are stored once, end to end in one string.

/// An interned name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameId(pub u32);

/// A set of names, each with an id in order of first use.
#[derive(Clone, Debug, Default)]
pub struct Names {
    /// All names, end to end.
    text: String,
    /// Where each name ends in `text`; it starts where the previous one ends.
    ends: Vec<u32>,
    /// Open-addressed hash table of ids, `EMPTY` where unused. Its length is
    /// a power of two, kept at least twice the number of names.
    slots: Vec<u32>,
}

const EMPTY: u32 = u32::MAX;

impl Names {
    pub fn new() -> Names {
        Names::default()
    }

    /// The id for `name`, adding it if new.
    pub fn intern(&mut self, name: &str) -> NameId {
        if self.slots.len() < 2 * (self.ends.len() + 1) {
            self.grow();
        }
        let mask = self.slots.len() - 1;
        let mut i = hash(name) as usize & mask;
        loop {
            let id = self.slots[i];
            if id == EMPTY {
                break;
            }
            if self.get(NameId(id)) == name {
                return NameId(id);
            }
            i = (i + 1) & mask;
        }
        let id = self.ends.len() as u32;
        self.text.push_str(name);
        self.ends.push(self.text.len() as u32);
        self.slots[i] = id;
        NameId(id)
    }

    /// The id for `name` if it's interned.
    pub fn find(&self, name: &str) -> Option<NameId> {
        if self.slots.is_empty() {
            return None;
        }
        let mask = self.slots.len() - 1;
        let mut i = hash(name) as usize & mask;
        loop {
            let id = self.slots[i];
            if id == EMPTY {
                return None;
            }
            if self.get(NameId(id)) == name {
                return Some(NameId(id));
            }
            i = (i + 1) & mask;
        }
    }

    pub fn get(&self, id: NameId) -> &str {
        let i = id.0 as usize;
        let start = if i == 0 { 0 } else { self.ends[i - 1] as usize };
        &self.text[start..self.ends[i] as usize]
    }

    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    fn grow(&mut self) {
        let size = (self.slots.len() * 2).max(64);
        self.slots = vec![EMPTY; size];
        let mask = size - 1;
        for id in 0..self.ends.len() as u32 {
            let mut i = hash(self.get(NameId(id))) as usize & mask;
            while self.slots[i] != EMPTY {
                i = (i + 1) & mask;
            }
            self.slots[i] = id;
        }
    }
}

/// FNV-1a, which is simple and good enough for short names.
fn hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_gives_stable_ids() {
        let mut names = Names::new();
        let words: Vec<String> = (0..200).map(|i| format!("n{i}")).collect();
        let ids: Vec<NameId> = words.iter().map(|w| names.intern(w)).collect();
        for (word, &id) in words.iter().zip(&ids) {
            assert_eq!(names.intern(word), id);
            assert_eq!(names.get(id), word);
            assert_eq!(names.find(word), Some(id));
        }
        assert_eq!(names.find("missing"), None);
        assert_eq!(names.len(), 200);
        assert_eq!(names.intern(""), NameId(200));
        assert_eq!(names.get(NameId(200)), "");
    }
}
