// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::{io, string::String};

use resvg::usvg;
use roxmltree::{Document, Node, NodeType};

use crate::transition::{Transition, Tree};

pub(crate) fn parse_tsvg<R: io::Read>(mut source: R) -> anyhow::Result<Tree> {
    let mut s = String::new();
    source.read_to_string(&mut s)?;

    let doc = Document::parse(&s)?;
    let root = doc.root_element();
    let root_name = root.tag_name().name();
    if root_name != "transitions" {
        return Err(anyhow::anyhow!("unexpected root element {}", root_name));
    }

    let mut transitions = Vec::new();
    for (i, c) in root.children().enumerate() {
        let node_type = c.node_type();
        match node_type {
            NodeType::Text => {
                let text = c.text().unwrap().trim();
                if text.is_empty() {
                    continue;
                }

                return Err(anyhow::anyhow!("unexpected text node"));
            }

            NodeType::Element => {
                let name = c.tag_name().name();
                if name != "transition" {
                    return Err(anyhow::anyhow!("unexpected element {}", name));
                }

                transitions.push(parse_transition(i, &c)?);
            }

            _ => {
                return Err(anyhow::anyhow!("unexpected node type {:?}", node_type));
            }
        }
    }

    Ok(Tree::new(transitions))
}

fn parse_transition(idx: usize, node: &Node) -> anyhow::Result<Transition> {
    let time_in = node
        .attribute("time-in")
        .map(|v| v.parse::<u64>())
        .transpose()?
        .ok_or(anyhow::anyhow!("no time-in attribute in transition"))?;

    let time_out = node
        .attribute("time-out")
        .map(|v| v.parse::<u64>())
        .transpose()?;

    let index = node
        .attribute("index")
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(idx);

    let tree = parse_svg(node)?;
    Ok(Transition {
        time_in,
        time_out,
        index,
        tree,
    })
}

fn parse_svg(transition_node: &Node) -> anyhow::Result<usvg::Tree> {
    match transition_node.first_child() {
        None => return Err(anyhow::anyhow!("missing SVG data in transition")),

        Some(c) => {
            let node_type = c.node_type();
            if node_type != NodeType::Text {
                return Err(anyhow::anyhow!(
                    "unexpected {:?} child node in transition",
                    node_type
                ));
            }

            if c.has_siblings() {
                return Err(anyhow::anyhow!("unexpeted multiple children in transition"));
            }

            let text = c.text().unwrap().trim();
            if text.is_empty() {
                Err(anyhow::anyhow!("empty SVG data in transition"))
            } else {
                let tree = usvg::Tree::from_str(text, &super::RESVG_OPTIONS.usvg)?;
                Ok(tree)
            }
        }
    }
}
