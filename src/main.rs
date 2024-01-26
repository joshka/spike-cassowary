use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

use cassowary::{strength::*, WeightedRelation::*, *};
use color_eyre::{eyre::eyre, Result};
use itertools::Itertools;

pub struct Layout {
    solver: Solver,
    area_element: Element,
    elements: Vec<Element>,
    values: HashMap<Variable, f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct Element {
    x: Variable,
    y: Variable,
    width: Variable,
    height: Variable,
}

#[derive(Clone, Copy)]
pub struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Layout {
    pub fn new(area: Rect) -> Result<Self> {
        let mut solver = Solver::new();
        let area_element = Element::new();

        // Edit variables can't be added with strength = REQUIRED
        // see https://github.com/dylanede/cassowary-rs/issues/2
        const EDIT_VARIABLE_STRENGTH: f64 = REQUIRED - 1.0;
        solver
            .add_edit_variable(area_element.x, EDIT_VARIABLE_STRENGTH)
            .map_err(|e| eyre!("failed to add x as edit variable: {e:?}"))?;
        solver
            .add_edit_variable(area_element.y, EDIT_VARIABLE_STRENGTH)
            .map_err(|e| eyre!("failed to add y as edit variable: {e:?}"))?;
        solver
            .add_edit_variable(area_element.width, EDIT_VARIABLE_STRENGTH)
            .map_err(|e| eyre!("failed to add width as edit variable: {e:?}"))?;
        solver
            .add_edit_variable(area_element.height, EDIT_VARIABLE_STRENGTH)
            .map_err(|e| eyre!("failed to add height as edit variable: {e:?}"))?;
        solver
            .suggest_value(area_element.x, area.x)
            .map_err(|e| eyre!("failed to set x: {e:?}"))?;
        solver
            .suggest_value(area_element.y, area.y)
            .map_err(|e| eyre!("failed to set y: {e:?}"))?;
        solver
            .suggest_value(area_element.width, area.width)
            .map_err(|e| eyre!("failed to set width: {e:?}"))?;
        solver
            .suggest_value(area_element.height, area.height)
            .map_err(|e| eyre!("failed to set height: {e:?}"))?;

        solver
            .add_constraints(&[
                area_element.x | GE(REQUIRED) | 0.0,
                area_element.y | GE(REQUIRED) | 0.0,
                area_element.width | GE(REQUIRED) | 0.0,
                area_element.height | GE(REQUIRED) | 0.0,
            ])
            .map_err(|e| eyre!("failed to add generic area constraints: {e:?}"))?;

        let layout = Layout {
            solver,
            area_element,
            elements: Vec::new(),
            values: HashMap::new(),
        };
        Ok(layout)
    }

    /// stores the rects in the layout and adds constraints to the solver
    pub fn add_element(&mut self, rect: Element) -> Result<()> {
        self.elements.push(rect);
        self.add_constraints(&[
            rect.left() | GE(REQUIRED) | self.area_element.left(),
            rect.top() | GE(REQUIRED) | self.area_element.top(),
            rect.right() | LE(REQUIRED) | self.area_element.bottom(),
            rect.bottom() | LE(REQUIRED) | self.area_element.bottom(),
        ])
    }

    /// adds multiple constraints to the solver
    pub fn add_constraints(&mut self, constraints: &[Constraint]) -> Result<()> {
        self.solver
            .add_constraints(constraints)
            .map_err(|e| eyre!("failed to add constraints: {e:?}"))
    }

    /// adds a single constraint to the solver
    pub fn add_constraint(&mut self, constraint: Constraint) -> Result<()> {
        self.solver
            .add_constraint(constraint)
            .map_err(|e| eyre!("failed to add constraint: {e:?}"))
    }

    /// fetches the values of the variables from the solver and stores them in the layout
    /// returns the rects of the elements
    pub fn get_rects(&mut self) -> Vec<Rect> {
        let mut rects = Vec::new();
        let changes = self.solver.fetch_changes();
        self.values.extend(changes.iter().copied());
        for element in self.elements.iter() {
            rects.push(Rect {
                x: self.value(element.x),
                y: self.value(element.y),
                width: self.value(element.width),
                height: self.value(element.height),
            });
        }
        rects
    }

    /// helper function to get the value of a variable from the solver
    fn value(&self, variable: Variable) -> f64 {
        self.values.get(&variable).copied().unwrap_or(0.0)
    }
}

impl Element {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Element {
            x: Variable::new(),
            y: Variable::new(),
            width: Variable::new(),
            height: Variable::new(),
        }
    }

    fn left(&self) -> Variable {
        self.x
    }

    fn right(&self) -> Expression {
        self.x + self.width
    }

    fn top(&self) -> Variable {
        self.y
    }

    fn bottom(&self) -> Expression {
        self.y + self.height
    }

    pub fn precedes_horizontally(&self, other: Element) -> Constraint {
        self.right() | EQ(REQUIRED) | other.left()
    }

    pub fn precedes_vertically(&self, other: Element) -> Constraint {
        self.bottom() | EQ(REQUIRED) | other.top()
    }

    pub fn has_width(&self, width: f64) -> Constraint {
        self.width | EQ(STRONG) | width
    }

    pub fn has_minimum_width(&self, width: f64) -> Constraint {
        self.width | GE(STRONG) | width
    }

    pub fn has_maximum_width(&self, width: f64) -> Constraint {
        self.width | LE(STRONG) | width
    }

    pub fn has_proportional_width(&self, other: Element, ratio: f64) -> Constraint {
        (self.width / ratio) | EQ(MEDIUM) | other.width
    }

    pub fn has_height(&self, height: f64) -> Constraint {
        self.height | EQ(STRONG) | height
    }

    pub fn has_minimum_height(&self, height: f64) -> Constraint {
        self.height | GE(STRONG) | height
    }

    pub fn has_maximum_height(&self, height: f64) -> Constraint {
        self.height | LE(STRONG) | height
    }

    pub fn has_proportional_height(&self, other: Element, ratio: f64) -> Constraint {
        (self.height / ratio) | EQ(MEDIUM) | other.height
    }
}

impl Debug for Rect {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rect {{ x: {:4.1}, y: {:4.1}, width: {:4.1}, height: {:4.1} }}",
            self.x, self.y, self.width, self.height
        )
    }
}

fn main() -> Result<()> {
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width: 50.0,
        height: 50.0,
    };

    let mut layout = Layout::new(area)?;
    let elements = [Element::new(), Element::new(), Element::new()];

    for element in elements.iter() {
        layout.add_element(*element)?;
    }

    let widths = [60.0, 30.0, 10.0];

    // set the width of each element
    for (&element, width) in elements.iter().zip(widths.iter()) {
        layout.add_constraint(element.has_width(*width))?;
    }

    // arrange elements horizontally
    for (&left, &right) in elements.iter().tuple_windows() {
        layout.add_constraint(left.precedes_horizontally(right))?;
    }

    // ensure that all elements are scaled proportionally if there is not enough space remaining
    for ((left, left_width), (right, right_width)) in
        elements.iter().zip(widths.iter()).tuple_windows()
    {
        layout.add_constraint(left.has_proportional_width(*right, *left_width / *right_width))?;
    }

    println!("widths: {:?}", widths);
    println!("area:\n    {:?}", area);
    println!("rects: {:#?}", layout.get_rects());
    // prints:
    // widths: [60.0, 30.0, 10.0]
    // area:
    //     Rect { x:  0.0, y:  0.0, width: 50.0, height: 50.0 }
    // rects: [
    //     Rect { x:  0.0, y:  0.0, width: 30.0, height: 50.0 },
    //     Rect { x: 30.0, y:  0.0, width: 15.0, height: 50.0 },
    //     Rect { x: 45.0, y:  0.0, width:  5.0, height: 50.0 },
    // ]

    Ok(())
}
