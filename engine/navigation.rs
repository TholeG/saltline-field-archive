use crate::physics::Collider;

#[derive(Clone, Copy)]
struct Candidate {
    cell: usize,
    priority: f64,
}

#[derive(Clone, Copy, Default)]
pub(super) struct GridGeometry {
    pub width: usize,
    pub height: usize,
    pub min_x: f64,
    pub min_z: f64,
    pub cell: f64,
    pub padding: f64,
}

impl GridGeometry {
    fn cell_coordinates(self, [x, z]: [f64; 2]) -> (f64, f64) {
        (
            ((x - self.min_x) / self.cell).floor(),
            ((z - self.min_z) / self.cell).floor(),
        )
    }

    fn index(self, x: i32, z: i32) -> Option<usize> {
        if x >= 0 && x < self.width as i32 && z >= 0 && z < self.height as i32 {
            Some(z as usize * self.width + x as usize)
        } else {
            None
        }
    }

    fn center(self, index: usize) -> [f64; 2] {
        let (x, z) = cell_xy(index, self.width);
        [
            self.min_x + (f64::from(x) + 0.5) * self.cell,
            self.min_z + (f64::from(z) + 0.5) * self.cell,
        ]
    }
}

#[derive(Default)]
pub(super) struct Navigation {
    geometry: GridGeometry,
    dirty: bool,
    grid: Vec<u8>,
    cost: Vec<f32>,
    parent: Vec<i32>,
    visited: Vec<u32>,
    closed: Vec<u32>,
    generation: u32,
    queue: Vec<Candidate>,
    path: Vec<i32>,
    waypoints: Vec<[f64; 2]>,
}

fn cell_xy(cell: usize, width: usize) -> (i32, i32) {
    ((cell % width) as i32, (cell / width) as i32)
}

// The JavaScript heap's equal-priority order influences which waypoints are chosen.
impl Navigation {
    pub fn new(geometry: GridGeometry) -> Self {
        let length = geometry.width * geometry.height;
        Self {
            geometry,
            dirty: true,
            grid: vec![0; length],
            cost: vec![0.0; length],
            parent: vec![-1; length],
            visited: vec![0; length],
            closed: vec![0; length],
            ..Self::default()
        }
    }

    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    pub fn prepare(&mut self, colliders: &[Collider], quay: f64, step_height: f64) {
        if !self.dirty {
            return;
        }
        let geometry = self.geometry;
        let GridGeometry {
            width,
            height,
            padding,
            ..
        } = geometry;
        let sea_start = (1..width - 1)
            .find(|&column| geometry.min_x + (column as f64 + 0.5) * geometry.cell > quay - padding)
            .unwrap_or(width - 1);
        for row in self.grid.chunks_exact_mut(width) {
            row.fill(0);
            row[0] = 1;
            row[sea_start..].fill(1);
        }
        self.grid[..width].fill(1);
        self.grid[(height - 1) * width..].fill(1);

        for collider in colliders {
            if collider.no_nav
                || collider.bullet_only
                || collider.max[1] < step_height + 0.05
                || collider.min[1] > 1.5
            {
                continue;
            }
            let (left, top) =
                geometry.cell_coordinates([collider.min[0] - padding, collider.min[2] - padding]);
            let (right, bottom) =
                geometry.cell_coordinates([collider.max[0] + padding, collider.max[2] + padding]);
            if [left, top, right, bottom]
                .iter()
                .any(|coordinate| coordinate.is_nan())
            {
                continue;
            }
            let left = left.max(0.0) as i32;
            let right = right.min((width - 1) as f64) as i32;
            let top = top.max(0.0) as i32;
            let bottom = bottom.min((height - 1) as f64) as i32;
            if left > right || top > bottom {
                continue;
            }
            for row in top as usize..=bottom as usize {
                let offset = row * width;
                self.grid[offset + left as usize..=offset + right as usize].fill(1);
            }
        }
        self.dirty = false;
    }

    pub fn grid(&self) -> &[u8] {
        &self.grid
    }

    fn cell(&self, point: [f64; 2]) -> Option<usize> {
        let (x, z) = self.geometry.cell_coordinates(point);
        if !x.is_finite() || !z.is_finite() {
            return None;
        }
        self.geometry.index(x as i32, z as i32)
    }

    fn nearest(&self, point: [f64; 2]) -> Option<usize> {
        let geometry = self.geometry;
        let (x, z) = geometry.cell_coordinates(point);
        if x.is_nan() || z.is_nan() {
            return None;
        }
        let x = (x as i32).clamp(0, geometry.width as i32 - 1);
        let z = (z as i32).clamp(0, geometry.height as i32 - 1);
        let center = geometry.index(x, z).unwrap();
        if self.grid[center] == 0 {
            return Some(center);
        }
        for ring in 1_i32..=12 {
            let mut nearest = None;
            let mut distance = i32::MAX;
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs().max(dz.abs()) != ring {
                        continue;
                    }
                    if let Some(cell) = geometry.index(x + dx, z + dz) {
                        let candidate_distance = dx * dx + dz * dz;
                        if self.grid[cell] == 0 && candidate_distance < distance {
                            distance = candidate_distance;
                            nearest = Some(cell);
                        }
                    }
                }
            }
            if nearest.is_some() {
                return nearest;
            }
        }
        None
    }

    pub fn is_walkable(&self, point: [f64; 2]) -> bool {
        self.cell(point).is_some_and(|cell| self.grid[cell] == 0)
    }

    pub fn world_line(&self, start: [f64; 2], goal: [f64; 2]) -> bool {
        match (self.cell(start), self.cell(goal)) {
            (Some(start), Some(goal)) => self.clear_line(self.geometry.width, start, goal),
            _ => false,
        }
    }

    pub fn sample(
        &self,
        [x, z]: [f64; 2],
        radius: f64,
        angle_sample: f64,
        radius_sample: f64,
    ) -> Option<[f64; 2]> {
        let angle = angle_sample * 6.2832;
        let distance = radius_sample.sqrt() * radius;
        let point = [x + angle.cos() * distance, z + angle.sin() * distance];
        self.is_walkable(point).then_some(point)
    }

    pub fn nearest_point(&self, point: [f64; 2]) -> [f64; 2] {
        self.nearest(point)
            .map_or(point, |cell| self.geometry.center(cell))
    }

    pub fn world_path(&mut self, start: [f64; 2], target: [f64; 2]) -> bool {
        self.waypoints.clear();
        let (Some(start), Some(goal)) = (self.nearest(start), self.nearest(target)) else {
            return false;
        };
        if start == goal {
            self.waypoints.push(target);
            return true;
        }
        let GridGeometry { width, height, .. } = self.geometry;
        if !self.search(width, height, start, goal) {
            return false;
        }
        self.path.clear();
        let mut cell = goal as i32;
        while cell >= 0 {
            self.path.push(cell);
            cell = self.parent[cell as usize];
        }
        self.path.reverse();
        let mut current = 0;
        while current + 1 < self.path.len() {
            let mut next = current + 1;
            for candidate in (current + 2..self.path.len()).rev() {
                if self.clear_line(
                    width,
                    self.path[current] as usize,
                    self.path[candidate] as usize,
                ) {
                    next = candidate;
                    break;
                }
            }
            let cell = self.path[next] as usize;
            self.waypoints.push(self.geometry.center(cell));
            current = next;
        }
        if self.cell(target) == Some(goal) {
            *self.waypoints.last_mut().unwrap() = target;
        }
        true
    }

    pub fn waypoints(&self) -> &[[f64; 2]] {
        &self.waypoints
    }

    fn push(&mut self, candidate: Candidate) {
        let mut index = self.queue.len();
        self.queue.push(candidate);
        while index > 0 {
            let parent = (index - 1) >> 1;
            if self.queue[parent].priority <= candidate.priority {
                break;
            }
            self.queue[index] = self.queue[parent];
            index = parent;
        }
        self.queue[index] = candidate;
    }

    fn pop(&mut self) -> Option<usize> {
        let first = self.queue.first()?.cell;
        let last = self.queue.pop().unwrap();
        if self.queue.is_empty() {
            return Some(first);
        }
        let mut index = 0;
        loop {
            let mut child = index * 2 + 1;
            if child >= self.queue.len() {
                break;
            }
            if child + 1 < self.queue.len()
                && self.queue[child + 1].priority < self.queue[child].priority
            {
                child += 1;
            }
            if self.queue[child].priority >= last.priority {
                break;
            }
            self.queue[index] = self.queue[child];
            index = child;
        }
        self.queue[index] = last;
        Some(first)
    }

    fn search(&mut self, width: usize, height: usize, start: usize, goal: usize) -> bool {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.visited.fill(0);
            self.closed.fill(0);
            self.generation = 1;
        }
        let stamp = self.generation;
        self.queue.clear();
        let (goal_x, goal_z) = cell_xy(goal, width);
        let estimate = |x: i32, z: i32| {
            let dx = f64::from((x - goal_x).abs());
            let dz = f64::from((z - goal_z).abs());
            dx + dz + (1.4142 - 2.0) * dx.min(dz)
        };
        let (x, z) = cell_xy(start, width);
        self.cost[start] = 0.0;
        self.parent[start] = -1;
        self.visited[start] = stamp;
        self.push(Candidate {
            cell: start,
            priority: estimate(x, z),
        });
        let mut explored = 0;
        while let Some(cell) = self.pop() {
            if cell == goal {
                break;
            }
            if self.closed[cell] == stamp {
                continue;
            }
            self.closed[cell] = stamp;
            explored += 1;
            if explored > 9000 {
                return false;
            }
            let (x, z) = cell_xy(cell, width);
            for dz in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dz == 0 {
                        continue;
                    }
                    let (next_x, next_z) = (x + dx, z + dz);
                    if next_x < 0 || next_z < 0 || next_x >= width as i32 || next_z >= height as i32
                    {
                        continue;
                    }
                    let next = next_z as usize * width + next_x as usize;
                    let diagonal = dx != 0 && dz != 0;
                    if self.grid[next] != 0
                        || self.closed[next] == stamp
                        || (diagonal
                            && (self.grid[z as usize * width + next_x as usize] != 0
                                || self.grid[next_z as usize * width + x as usize] != 0))
                    {
                        continue;
                    }
                    let cost = f64::from(self.cost[cell]) + if diagonal { 1.4142 } else { 1.0 };
                    if self.visited[next] != stamp || cost < f64::from(self.cost[next]) {
                        self.cost[next] = cost as f32;
                        self.visited[next] = stamp;
                        self.parent[next] = cell as i32;
                        self.push(Candidate {
                            cell: next,
                            priority: cost + estimate(next_x, next_z),
                        });
                    }
                }
            }
        }
        self.visited[goal] == stamp
    }

    fn clear_line(&self, width: usize, start: usize, goal: usize) -> bool {
        let (mut x, mut z) = cell_xy(start, width);
        let (end_x, end_z) = cell_xy(goal, width);
        let (dx, dz) = ((end_x - x).abs(), (end_z - z).abs());
        let (step_x, step_z) = (
            if x < end_x { 1 } else { -1 },
            if z < end_z { 1 } else { -1 },
        );
        let index = |x: i32, z: i32| z as usize * width + x as usize;
        let mut balance = dx - dz;
        for _ in 0..600 {
            if self.grid[index(x, z)] != 0 {
                return false;
            }
            if x == end_x && z == end_z {
                return true;
            }
            let error = 2 * balance;
            if error > -dz && error < dx {
                if self.grid[index(x + step_x, z)] != 0 || self.grid[index(x, z + step_z)] != 0 {
                    return false;
                }
                balance += dx - dz;
                x += step_x;
                z += step_z;
            } else if error > -dz {
                balance -= dz;
                x += step_x;
            } else {
                balance += dx;
                z += step_z;
            }
        }
        false
    }
}
