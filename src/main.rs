#![allow(dead_code)]
use std::{
    collections::{hash_map::RandomState, HashMap},
    fmt::{Display, Formatter},
};

use cassowary::{strength::*, WeightedRelation::*, *};
use color_eyre::{eyre::eyre, Result};

pub struct Layout {
    solver: Solver,
    width_variable: Variable,
    height_variable: Variable,
    rects: Vec<Rect>,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    // this could be split into a struct for the variables and a struct for the values
    x_variable: Variable,
    y_variable: Variable,
    width_variable: Variable,
    height_variable: Variable,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Display for Rect {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rect {{ x: {:4.1}, y: {:4.1}, width: {:4.1}, height: {:4.1} }}",
            self.x, self.y, self.width, self.height
        )
    }
}

impl Layout {
    pub fn new() -> Result<Self> {
        let mut solver = Solver::new();
        let width = Variable::new();
        let height = Variable::new();
        solver
            .add_edit_variable(width, REQUIRED - 1.0) // https://github.com/dylanede/cassowary-rs/issues/2
            .map_err(|e| eyre!("failed to add width: {e:?}"))?;
        solver
            .add_edit_variable(height, REQUIRED - 1.0)
            .map_err(|e| eyre!("failed to add height: {e:?}"))?;
        solver
            .add_constraints(&[width | GE(REQUIRED) | 0.0, height | GE(REQUIRED) | 0.0])
            .map_err(|e| eyre!("failed to add constraints: {e:?}"))?;
        Ok(Layout {
            solver,
            width_variable: width,
            height_variable: height,
            rects: Vec::new(),
            width: 0.0,
            height: 0.0,
        })
    }

    pub fn add_rect(&mut self, rect: Rect) -> Result<()> {
        self.add_constraints(&[
            rect.x_variable | GE(REQUIRED) | 0.0,
            rect.y_variable | GE(REQUIRED) | 0.0,
            rect.x_variable + rect.width_variable | LE(REQUIRED) | self.width_variable,
            rect.y_variable + rect.height_variable | LE(REQUIRED) | self.height_variable,
        ])?;
        self.rects.push(rect);
        Ok(())
    }

    pub fn add_constraints(&mut self, constraints: &[Constraint]) -> Result<()> {
        self.solver
            .add_constraints(constraints)
            .map_err(|e| eyre!("failed to add constraints: {e:?}"))?;
        Ok(())
    }

    pub fn add_constraint(&mut self, constraint: Constraint) -> Result<()> {
        self.solver
            .add_constraint(constraint)
            .map_err(|e| eyre!("failed to add constraint: {e:?}"))?;
        Ok(())
    }

    pub fn set_size(&mut self, width: f64, height: f64) -> Result<()> {
        self.solver
            .suggest_value(self.width_variable, width)
            .map_err(|e| eyre!("failed to set width: {e:?}"))?;
        self.solver
            .suggest_value(self.height_variable, height)
            .map_err(|e| eyre!("failed to set height: {e:?}"))?;
        let changes = self.solver.fetch_changes();
        let map: HashMap<_, _, RandomState> = HashMap::from_iter(changes.iter().copied());
        for rect in &mut self.rects {
            rect.x = *map.get(&rect.x_variable).unwrap_or(&0.0);
            rect.y = *map.get(&rect.y_variable).unwrap_or(&0.0);
            rect.width = *map.get(&rect.width_variable).unwrap_or(&0.0);
            rect.height = *map.get(&rect.height_variable).unwrap_or(&0.0);
        }
        Ok(())
    }
}

impl Rect {
    pub fn new() -> Self {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            x_variable: Variable::new(),
            y_variable: Variable::new(),
            width_variable: Variable::new(),
            height_variable: Variable::new(),
        }
    }

    fn precedes_horizontally(&self, other: Rect) -> Constraint {
        self.x_variable + self.width_variable | EQ(REQUIRED) | other.x_variable
    }

    fn precedes_vertically(&self, other: Rect) -> Constraint {
        self.y_variable + self.height_variable | EQ(REQUIRED) | other.y_variable
    }

    pub fn has_width(&self, width: f64) -> Constraint {
        self.width_variable | EQ(STRONG) | width
    }

    fn has_proportional_width(&self, other: Rect, ratio: f64) -> Constraint {
        self.width_variable / ratio | EQ(MEDIUM) | other.width_variable
    }
}

fn main() -> Result<()> {
    let mut layout = Layout::new()?;
    let rects = vec![Rect::new(), Rect::new(), Rect::new()];
    for rect in rects.iter() {
        layout.add_rect(*rect)?;
    }

    layout.add_constraint(rects[0].precedes_horizontally(rects[1]))?;
    layout.add_constraint(rects[1].precedes_horizontally(rects[2]))?;
    layout.add_constraint(rects[0].has_width(60.0))?;
    layout.add_constraint(rects[1].has_width(30.0))?;
    layout.add_constraint(rects[2].has_width(10.0))?;
    layout.add_constraint(rects[0].has_proportional_width(rects[1], 60.0 / 30.0))?;
    layout.add_constraint(rects[1].has_proportional_width(rects[2], 30.0 / 10.0))?;

    layout.set_size(50.0, 50.0)?;

    println!(
        "Layout {{ width: {}, height: {} }}",
        layout.width, layout.height
    );
    for rect in &layout.rects {
        println!("{}", rect);
    }

    Ok(())
}
