// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;
use std::collections::VecDeque;

use resvg::usvg;

pub(crate) struct Transition {
    pub(crate) time_in: u64,
    pub(crate) time_out: Option<u64>,
    pub(crate) index: usize,
    pub(crate) tree: usvg::Tree,
}

// Tree implementation inspired by https://github.com/main--/rust-intervaltree

pub(crate) struct Tree {
    nodes: Vec<TreeNode>,
}

struct TreeNode {
    transition: Transition,
    max: Option<u64>,
}

impl Tree {
    pub(crate) fn new(transitions: Vec<Transition>) -> Tree {
        let mut nodes: Vec<TreeNode> = transitions
            .into_iter()
            .map(|transition| {
                let max = transition.time_out;
                TreeNode { transition, max }
            })
            .collect();

        nodes.sort_by(|a, b| Self::cmp_transitions(&a.transition, &b.transition));
        if !nodes.is_empty() {
            Self::update_max(&mut nodes);
        }

        Tree { nodes }
    }

    pub(crate) fn search(&self, ts_millis: f64) -> Vec<&Transition> {
        let mut result = vec![];
        let mut stack = VecDeque::new();
        if !self.nodes.is_empty() {
            stack.push_back((0, self.nodes.len()));
        }

        while let Some((s, l)) = stack.pop_back() {
            let idx = s + l / 2;
            let node = &self.nodes[idx];
            if node.max.map(|m| ts_millis < m as f64).unwrap_or(true) {
                let ls = idx - s;
                if ls > 0 {
                    stack.push_back((s, ls));
                }

                if ts_millis >= node.transition.time_in as f64 {
                    let rs = l + s - idx - 1;
                    if rs > 0 {
                        stack.push_back((idx + 1, rs));
                    }

                    if node
                        .transition
                        .time_out
                        .map(|o| ts_millis < o as f64)
                        .unwrap_or(true)
                    {
                        result.push(&node.transition);
                    }
                }
            }
        }

        result.sort_by(|a, b| a.index.cmp(&b.index));
        result
    }

    fn update_max(nodes: &mut [TreeNode]) -> Option<u64> {
        let mid = nodes.len() / 2;
        if nodes.len() > 1 {
            let (left, rest) = nodes.split_at_mut(mid);
            if !left.is_empty() {
                let left_max = Self::update_max(left);
                rest[0].max = Self::max(rest[0].max, left_max);
            }

            let (rest, right) = nodes.split_at_mut(mid + 1);
            if !right.is_empty() {
                let right_max = Self::update_max(right);
                rest[mid].max = Self::max(rest[mid].max, right_max);
            }
        }

        nodes[mid].max
    }

    fn cmp_transitions(a: &Transition, b: &Transition) -> Ordering {
        let ordering = a.time_in.cmp(&b.time_in);
        if ordering == Ordering::Equal {
            match (a.time_out, b.time_out) {
                (None, None) => Ordering::Equal,
                (None, _) => Ordering::Greater,
                (_, None) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(&b),
            }
        } else {
            ordering
        }
    }

    fn max(a: Option<u64>, b: Option<u64>) -> Option<u64> {
        match (a, b) {
            (None, _) => None,
            (_, None) => None,
            (Some(a), Some(b)) => Some(a.max(b)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let tree = Tree::new(vec![]);
        assert!(tree.search(0.0).is_empty());
    }

    #[test]
    fn it_works() {
        let svg_tree = usvg::Tree::create(usvg::Svg {
            size: usvg::Size::new(640.0, 360.0).unwrap(),
            view_box: usvg::ViewBox {
                rect: usvg::Rect::new(0.0, 0.0, 640.0, 360.0).unwrap(),
                aspect: usvg::AspectRatio::default(),
            },
        });

        let transitions = vec![
            Transition {
                time_in: 4,
                time_out: Some(8),
                index: 0,
                tree: svg_tree.clone(),
            },
            Transition {
                time_in: 2,
                time_out: Some(10),
                index: 1,
                tree: svg_tree.clone(),
            },
            Transition {
                time_in: 10,
                time_out: None,
                index: 2,
                tree: svg_tree.clone(),
            },
            Transition {
                time_in: 10,
                time_out: Some(12),
                index: 3,
                tree: svg_tree,
            },
        ];

        let tree = Tree::new(transitions);
        assert!(tree.search(0.0).is_empty());

        let result = tree.search(3.0);
        assert_eq!(1, result.len());
        assert_eq!(2, result[0].time_in);
        assert_eq!(Some(10), result[0].time_out);

        let result = tree.search(5.0);
        assert_eq!(2, result.len());
        assert_eq!(4, result[0].time_in);
        assert_eq!(Some(8), result[0].time_out);
        assert_eq!(2, result[1].time_in);
        assert_eq!(Some(10), result[1].time_out);

        let result = tree.search(8.0);
        assert_eq!(1, result.len());
        assert_eq!(2, result[0].time_in);
        assert_eq!(Some(10), result[0].time_out);

        let result = tree.search(10.0);
        assert_eq!(2, result.len());
        assert_eq!(10, result[0].time_in);
        assert_eq!(None, result[0].time_out);
        assert_eq!(10, result[1].time_in);
        assert_eq!(Some(12), result[1].time_out);

        let result = tree.search(12.0);
        assert_eq!(1, result.len());
        assert_eq!(10, result[0].time_in);
        assert_eq!(None, result[0].time_out);

        let result = tree.search(1000.0);
        assert_eq!(1, result.len());
        assert_eq!(10, result[0].time_in);
        assert_eq!(None, result[0].time_out);
    }
}
