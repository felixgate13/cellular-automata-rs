use std::*;
use std::ops::*;


pub enum RuleGroup {
    Single(i8),
    Multiple(Vec<i8>)
}
pub enum RuleType {
    Survival,
    Birth,
    State,
    Neighbor
}

pub struct Rule {
    ruletype: RuleType,
    rulegroup: RuleGroup
}

impl Rule {
    pub fn get_binary_rule(&self) -> [bool; 27] {
        let mut out: [bool; 27] = [false; 27];
        match &self.rulegroup {
            RuleGroup::Single(n) => {
                out[*n as usize] = true;
                return out
            },

            RuleGroup::Multiple(v) => {
                v.iter().map(|i| out[*i as usize] = true);
                out
            },

        }
    }
}