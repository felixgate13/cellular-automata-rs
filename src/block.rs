use ndarray::Array3;
use std::*;
use super::settings;
use cgmath::{prelude::*, num_traits::Num};
#[path = "graphics/wgpud/instance.rs"] mod instance;

#[derive(Debug, Clone)]
pub enum Method {
    VonNeumann, 
    Moore
}
/// an item containing all the cells to be passed to rendering and designating rules.
#[derive(Debug, Clone)]
pub struct Block {
    pub method: Method,
    pub edge_max: i16,
    pub step_in: i16,
    pub n_rule: [bool; 27],
    pub b_rule: [bool; 27],
    pub s_rule: i8,
    pub grid: Array3<i8>,
}


impl Block {
    ///initialise the start shape with 1ns, process initial neighbors and spit out the array for processing.
    pub fn get_fresh_grid(start_shape: settings::StartShape, edge:&i16, step_in: i16, s_rule: i8) -> Array3::<i8> {
        let edge_usize = *edge as usize;
        let mut grid = Array3::<i8>::zeros((edge_usize, edge_usize, edge_usize));

        match start_shape.shape {
            settings::Shape::Diamond => {
                //todo:
                //init with one as all thats needed for alive
                //return diamond shape
            },
            settings::Shape::Cube => {

                let draw_length = (*edge - (step_in * 2)) as i16 ;
                let instep = step_in;

                let max = draw_length + instep; 
                let min = instep;  

                if start_shape.is_hollow {
                    for x in min..max {
                        for y in min..max {
                            for z in min..max {
                                //for indexing
                                let max = max - 1;
                                //if x or y or z == hollow max or Min then draw
                                if x == min || x == max || y == min 
                                || y == max || z == min || z == max  {
                                    grid[[x as usize, y as usize, z as usize]] = s_rule ;
                                }
                            }
                        }
                    }

                } else {
                    //fill the whole thing
                    for x in instep..max {
                        for y in instep..max {
                            for z in instep..max {
                                grid[[x as usize, y as usize, z as usize]] = s_rule;
                            }
                        }
                    }
                }
            },
        }
    
        return grid
    }

    ///passes in a grid and makes changes based on given rules
    /// n_rule is how many to stay alive, b_rule for to be born, s_rule for how many game tics till dead
    pub fn update_grid(&mut self)  {

        let s_rule = self.s_rule - 1;
        let old_grid  = self.grid.clone();
        for x  in 1 as usize.. (self.edge_max - 2) as usize {
            for y in 1 as usize.. (self.edge_max - 2) as usize {
                for z in 1 as usize.. (self.edge_max - 2) as usize {
                    let neighbors: usize = Block::get_neighbors(&old_grid, x, y, z, &self.method);
                    let grid_val: i8 = old_grid[[x, y, z]];

                    match grid_val {
                        0 => {
                            //if dead be born if correct amount of neighbors
                            if self.b_rule[neighbors] == true {
                                self.grid[[x, y, z]] = s_rule;
                            } 
                        },
                        1 => {
                            if self.n_rule[neighbors] == false {
                                self.grid[[x, y, z]] = 0;
                            }
                        }
                        _ => {
                                self.grid[[x, y, z]] = (grid_val - 1) as i8;

                        }
                    }
                }
            }   
        }
        
    }
    
    //get sum of neighbors that == 0
    pub fn get_neighbors(grid: &Array3::<i8>, x: usize, y:usize, z:usize, method: &Method) -> usize {
        if y == 0 || x == 0 || z == 0 {
            panic!("Edges of the cube is out of bounds!!")
        }
        match method {
            //filter any neighbors thats value is alive (0) and collect.len()
            Method::Moore => {
                let params  = settings::TRANSLATIONS_MOORE;

                //messy?
                params.iter().filter(|p| grid[[(x as i8 +p[0]) as usize, (y as i8+p[1]) as usize, (z as i8+p[2]) as usize ]] > 0).collect::<Vec<&[i8; 3]>>().len()

            },
            Method::VonNeumann => {
                let params  = settings::TRANSLATIONS_VON;

                 params.iter().filter(|p| grid[[(x as i8 +p[0]) as usize, (y as i8+p[1]) as usize, (z as i8+p[2]) as usize ]] > 0).collect::<Vec<&[i8; 3]>>().len()
                
            },
        }



    }


}