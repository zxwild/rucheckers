//! Дерево партии: история ходов с вариантами.
//!
//! Каждый узел хранит позицию после хода и счётчики ничьей, накопленные по ветке,
//! ведущей к нему. Поэтому повторения позиций и лимиты ничьей всегда считаются
//! по той ветке, на которой находится узел, и не зависят от соседних вариантов.

use crate::board::Position;
use crate::moves::{legal_moves, Move};
use crate::rules::{GameState, Outcome};
use std::fmt;

/// Идентификатор узла дерева.
pub type NodeId = usize;

/// Узел дерева партии.
#[derive(Clone, Debug)]
pub struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    mv: Option<Move>,
    state: GameState,
    ply: u32,
    occurrences: u32,
    outcome: Option<Outcome>,
    preferred: usize,
}

impl Node {
    /// Родительский узел (у корня его нет).
    #[must_use]
    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Продолжения: первое — основное, остальные — варианты в порядке создания.
    #[must_use]
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// Ход, которым получена позиция (у корня его нет).
    #[must_use]
    pub const fn mv(&self) -> Option<&Move> {
        self.mv.as_ref()
    }

    /// Позиция и счётчики ничьей.
    #[must_use]
    pub const fn state(&self) -> &GameState {
        &self.state
    }

    /// Позиция после хода.
    #[must_use]
    pub const fn position(&self) -> &Position {
        &self.state.pos
    }

    /// Число полуходов от начала партии.
    #[must_use]
    pub const fn ply(&self) -> u32 {
        self.ply
    }

    /// Сколько раз позиция встретилась на ветке, считая этот узел.
    #[must_use]
    pub const fn occurrences(&self) -> u32 {
        self.occurrences
    }

    /// Результат, если партия в этом узле закончена.
    #[must_use]
    pub const fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Продолжение, по которому пойдёт переход «вперёд».
    #[must_use]
    pub fn preferred_child(&self) -> Option<NodeId> {
        self.children.get(self.preferred).copied()
    }
}

/// Ошибка изменения дерева.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeError {
    /// Узла с таким идентификатором нет.
    NoSuchNode,
    /// Партия в этом узле уже закончена.
    GameOver,
    /// Ход недопустим в позиции узла.
    IllegalMove,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoSuchNode => "no such node",
            Self::GameOver => "the game is over",
            Self::IllegalMove => "illegal move",
        })
    }
}

impl std::error::Error for TreeError {}

/// Дерево партии с указателем на текущую позицию.
#[derive(Clone, Debug)]
pub struct GameTree {
    nodes: Vec<Node>,
    current: NodeId,
}

impl Default for GameTree {
    fn default() -> Self {
        Self::new()
    }
}

impl GameTree {
    /// Корень дерева.
    pub const ROOT: NodeId = 0;

    /// Партия из начальной расстановки.
    #[must_use]
    pub fn new() -> Self {
        Self::from_position(Position::initial())
    }

    /// Партия из произвольной позиции.
    #[must_use]
    pub fn from_position(pos: Position) -> Self {
        let state = GameState::new(pos);
        let root = Node {
            parent: None,
            children: Vec::new(),
            mv: None,
            state,
            ply: 0,
            occurrences: 1,
            outcome: state.outcome(1),
            preferred: 0,
        };
        Self {
            nodes: vec![root],
            current: Self::ROOT,
        }
    }

    /// Число узлов, включая корень.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Дерево без единого хода.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    /// Узел по идентификатору.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Корневой узел.
    #[must_use]
    pub fn root(&self) -> &Node {
        &self.nodes[Self::ROOT]
    }

    /// Идентификатор текущего узла.
    #[must_use]
    pub const fn current(&self) -> NodeId {
        self.current
    }

    /// Текущий узел.
    #[must_use]
    pub fn current_node(&self) -> &Node {
        &self.nodes[self.current]
    }

    /// Adds a move to the node `parent` without changing the current position.
    ///
    /// If the node already has this move, the existing node is returned; otherwise a new
    /// variation is created and the earlier continuations are kept.
    ///
    /// # Errors
    ///
    /// Returns [`TreeError::NoSuchNode`] if `parent` is not in the tree,
    /// [`TreeError::GameOver`] if the game has ended at `parent`, and
    /// [`TreeError::IllegalMove`] if the move is not legal in its position.
    pub fn add_move(&mut self, parent: NodeId, mv: &Move) -> Result<NodeId, TreeError> {
        let node = self.nodes.get(parent).ok_or(TreeError::NoSuchNode)?;
        if let Some(existing) = node
            .children
            .iter()
            .copied()
            .find(|&c| self.nodes[c].mv.as_ref() == Some(mv))
        {
            return Ok(existing);
        }
        if node.outcome.is_some() {
            return Err(TreeError::GameOver);
        }
        if !legal_moves(&node.state.pos).contains(mv) {
            return Err(TreeError::IllegalMove);
        }
        let state = node.state.play(mv);
        let ply = node.ply + 1;
        let occurrences = self.count_occurrences(parent, &state);
        let id = self.nodes.len();
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            mv: Some(*mv),
            state,
            ply,
            occurrences,
            outcome: state.outcome(occurrences),
            preferred: 0,
        });
        self.nodes[parent].children.push(id);
        Ok(id)
    }

    /// Сколько раз позиция `state` встречается на ветке, заканчивающейся в `parent`,
    /// считая её саму. Повториться могут только позиции, разделённые тихими ходами
    /// дамок, поэтому достаточно просмотреть столько предков, сколько таких ходов
    /// сделано подряд.
    fn count_occurrences(&self, parent: NodeId, state: &GameState) -> u32 {
        let mut count = 1;
        let mut cursor = Some(parent);
        let mut left = state.counters.kings_only;
        while left > 0 {
            let Some(id) = cursor else { break };
            let node = &self.nodes[id];
            if node.state.pos == state.pos {
                count += 1;
            }
            cursor = node.parent;
            left -= 1;
        }
        count
    }

    /// Plays a move from the current position and goes to the new one.
    ///
    /// If the move differs from the continuations recorded so far, a new variation is
    /// created.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`GameTree::add_move`].
    pub fn play(&mut self, mv: &Move) -> Result<NodeId, TreeError> {
        let id = self.add_move(self.current, mv)?;
        self.go_to(id);
        Ok(id)
    }

    /// Переходит к узлу. Ветка, ведущая к нему, становится активной.
    pub fn go_to(&mut self, id: NodeId) -> bool {
        if id >= self.nodes.len() {
            return false;
        }
        self.current = id;
        let mut child = id;
        while let Some(parent) = self.nodes[child].parent {
            if let Some(index) = self.nodes[parent].children.iter().position(|&c| c == child) {
                self.nodes[parent].preferred = index;
            }
            child = parent;
        }
        true
    }

    /// На ход назад.
    pub fn back(&mut self) -> bool {
        match self.nodes[self.current].parent {
            Some(parent) => {
                self.current = parent;
                true
            }
            None => false,
        }
    }

    /// На ход вперёд по активной ветке.
    pub fn forward(&mut self) -> bool {
        match self.nodes[self.current].preferred_child() {
            Some(child) => {
                self.current = child;
                true
            }
            None => false,
        }
    }

    /// К началу партии.
    pub const fn go_start(&mut self) -> bool {
        let moved = self.current != Self::ROOT;
        self.current = Self::ROOT;
        moved
    }

    /// К концу активной ветки.
    pub fn go_end(&mut self) -> bool {
        let mut moved = false;
        while self.forward() {
            moved = true;
        }
        moved
    }

    /// Переходит к соседнему варианту: `step = 1` — следующий, `-1` — предыдущий.
    pub fn switch_variation(&mut self, step: isize) -> bool {
        let Some(parent) = self.nodes[self.current].parent else {
            return false;
        };
        let siblings = &self.nodes[parent].children;
        if siblings.len() < 2 {
            return false;
        }
        let Some(index) = siblings.iter().position(|&c| c == self.current) else {
            return false;
        };
        let count = siblings.len() as isize;
        let next = (index as isize + step).rem_euclid(count) as usize;
        let target = siblings[next];
        self.go_to(target)
    }

    /// Все варианты хода, которым получен узел (включая его самого).
    #[must_use]
    pub fn siblings(&self, id: NodeId) -> &[NodeId] {
        match self.nodes.get(id).and_then(|n| n.parent) {
            Some(parent) => &self.nodes[parent].children,
            None => &[],
        }
    }

    /// Путь от корня до узла включительно.
    #[must_use]
    pub fn path_to(&self, id: NodeId) -> Vec<NodeId> {
        let mut path = Vec::new();
        let mut cursor = if id < self.nodes.len() { Some(id) } else { None };
        while let Some(node) = cursor {
            path.push(node);
            cursor = self.nodes[node].parent;
        }
        path.reverse();
        path
    }

    /// Активная ветка: от корня через текущий узел до конца по предпочтительным продолжениям.
    #[must_use]
    pub fn active_line(&self) -> Vec<NodeId> {
        let mut line = self.path_to(self.current);
        let mut cursor = self.current;
        while let Some(child) = self.nodes[cursor].preferred_child() {
            line.push(child);
            cursor = child;
        }
        line
    }

    /// Последний узел основной линии (первые продолжения от корня).
    #[must_use]
    pub fn main_line_end(&self) -> NodeId {
        let mut cursor = Self::ROOT;
        while let Some(&child) = self.nodes[cursor].children.first() {
            cursor = child;
        }
        cursor
    }

    /// Позиции ветки, предшествующие узлу, которые ещё могут повториться
    /// (от более ранних к более поздним, без самого узла).
    #[must_use]
    pub fn history_before(&self, id: NodeId) -> Vec<Position> {
        let mut history = Vec::new();
        let Some(node) = self.nodes.get(id) else {
            return history;
        };
        let mut left = node.state.counters.kings_only;
        let mut cursor = node.parent;
        while left > 0 {
            let Some(parent) = cursor else { break };
            history.push(self.nodes[parent].state.pos);
            cursor = self.nodes[parent].parent;
            left -= 1;
        }
        history.reverse();
        history
    }

    /// Совпадают ли деревья по структуре: начальная позиция, ходы и порядок вариантов.
    /// Положение указателя текущей позиции не сравнивается.
    #[must_use]
    pub fn same_structure(&self, other: &Self) -> bool {
        if self.nodes.len() != other.nodes.len() || self.root().state != other.root().state {
            return false;
        }
        let mut stack = vec![(Self::ROOT, Self::ROOT)];
        while let Some((a, b)) = stack.pop() {
            let (na, nb) = (&self.nodes[a], &other.nodes[b]);
            if na.mv != nb.mv || na.children.len() != nb.children.len() || na.state != nb.state {
                return false;
            }
            stack.extend(na.children.iter().copied().zip(nb.children.iter().copied()));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notation::parse_move;

    fn play(tree: &mut GameTree, text: &str) -> NodeId {
        let legal = legal_moves(tree.current_node().position());
        let mv = parse_move(text, &legal).unwrap_or_else(|e| panic!("{text}: {e}"));
        tree.play(&mv).unwrap()
    }

    #[test]
    fn replaying_a_recorded_move_does_not_create_a_variation() {
        let mut tree = GameTree::new();
        let a = play(&mut tree, "c3-d4");
        tree.back();
        let b = play(&mut tree, "c3-d4");
        assert_eq!(a, b);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree.root().children().len(), 1);
    }

    #[test]
    fn illegal_moves_and_bad_ids_are_rejected() {
        let mut tree = GameTree::new();
        let first = legal_moves(tree.root().position())[0];
        let id = tree.play(&first).unwrap();
        // Тот же ход уже недопустим: теперь очередь чёрных.
        assert_eq!(tree.add_move(id, &first), Err(TreeError::IllegalMove));
        assert_eq!(tree.add_move(999, &first), Err(TreeError::NoSuchNode));
        assert!(!tree.go_to(999));
        assert!(tree.get(999).is_none());
        assert!(tree.siblings(999).is_empty());
        assert!(tree.path_to(999).is_empty());
        assert!(tree.history_before(999).is_empty());
    }
}
