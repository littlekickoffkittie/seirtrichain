//! Core geometric primitives and fractal logic for siertrichain.
//! Defines the Point and Triangle structs, subdivision logic, and validation.

use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};

/// Coordinate type for high-precision geometric calculations.
pub type Coord = f64;
/// Tolerance for floating point comparisons to check for degeneracy/equality.
const GEOMETRIC_TOLERANCE: Coord = 1e-9; 

// ----------------------------------------------------------------------------
// 1.4 Coordinate System: Point
// ----------------------------------------------------------------------------

/// Represents a 2D point with high-precision coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
}

impl Point {
    /// Creates a new Point.
    pub fn new(x: Coord, y: Coord) -> Self {
        Point { x, y }
    }

    /// Calculates the midpoint between this point and another.
    pub fn midpoint(&self, other: &Point) -> Point {
        Point::new(
            (self.x + other.x) / 2.0,
            (self.y + other.y) / 2.0,
        )
    }

    /// Calculates a simple cryptographic hash of the point data.
    pub fn hash(&self) -> String {
        let data = format!("{:.15},{:.15}", self.x, self.y);
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// ----------------------------------------------------------------------------
// 1.3 Triangle Data Structure & Core Methods
// ----------------------------------------------------------------------------

/// Represents a triangle defined by three points (vertices).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Triangle {
    pub a: Point,
    pub b: Point,
    pub c: Point,
    pub parent_hash: Option<String>, 
}

impl Triangle {
    /// Creates a new Triangle from three vertices.
    pub fn new(a: Point, b: Point, c: Point, parent_hash: Option<String>) -> Self {
        Triangle { a, b, c, parent_hash }
    }

    /// Calculates the center point (centroid) of the triangle.

    /// Calculates the area of the triangle using the Shoelace formula.
    pub fn area(&self) -> Coord {
        let val = (self.a.x * (self.b.y - self.c.y) 
                 + self.b.x * (self.c.y - self.a.y) 
                 + self.c.x * (self.a.y - self.b.y))
                 .abs();
        val / 2.0
    }

    /// Calculates the unique cryptographic hash of the triangle.
    pub fn hash(&self) -> String {
        let mut hashes = vec![self.a.hash(), self.b.hash(), self.c.hash()];
        hashes.sort(); 
        
        let data = hashes.join("");
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // ------------------------------------------------------------------------
    // 1.6 Genesis Triangle Implementation
    // ------------------------------------------------------------------------

    /// Defines the canonical Genesis Triangle for the siertrichain.
    pub fn genesis() -> Self {
        const SQRT3: Coord = 1.7320508075688772;
        const HALF_SQRT3: Coord = 0.8660254037844386;
        const ONE_POINT_FIVE: Coord = 1.5;

        Triangle::new(
            Point::new(0.0, 0.0),
            Point::new(SQRT3, 0.0),
            Point::new(HALF_SQRT3, ONE_POINT_FIVE),
            None, 
        )
    }
    
    // ------------------------------------------------------------------------
    // 1.7 Subdivision Algorithm
    // ------------------------------------------------------------------------

    /// Subdivides the current triangle into three smaller, valid triangles.
    pub fn subdivide(&self) -> [Triangle; 3] {
        let mid_ab = self.a.midpoint(&self.b);
        let mid_bc = self.b.midpoint(&self.c);
        let mid_ca = self.c.midpoint(&self.a);

        let parent_hash = Some(self.hash());

        // Child 1 (A-mid_ab-mid_ca)
        let t1 = Triangle::new(self.a, mid_ab, mid_ca, parent_hash.clone());

        // Child 2 (mid_ab-B-mid_bc)
        let t2 = Triangle::new(mid_ab, self.b, mid_bc, parent_hash.clone());

        // Child 3 (mid_ca-mid_bc-C)
        let t3 = Triangle::new(mid_ca, mid_bc, self.c, parent_hash);
        
        [t1, t2, t3]
    }

    // ------------------------------------------------------------------------
    // 1.8 Geometric Validation
    // ------------------------------------------------------------------------

    /// Checks if the triangle is geometrically valid.
    /// This currently checks for non-degenerate triangles (Area > Tolerance).
    pub fn is_valid(&self) -> bool {
        // A valid triangle must have a non-zero area (i.e., not collinear points).
        self.area() > GEOMETRIC_TOLERANCE
    }
}


// ----------------------------------------------------------------------------
// Testing
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_triangle() -> Triangle {
        Triangle::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
            None,
        )
    }

    #[test]
    fn test_point_midpoint() {
        let p1 = Point::new(1.0, 1.0);
        let p2 = Point::new(5.0, 5.0);
        let midpoint = p1.midpoint(&p2);
        assert_eq!(midpoint, Point::new(3.0, 3.0));
    }

    #[test]
    fn test_triangle_area() {
        let t = setup_test_triangle();
        assert_eq!(t.area(), 50.0);
    }

    #[test]
    fn test_triangle_hash_is_canonical() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 4.0);
        let p3 = Point::new(5.0, 6.0);

        let t1 = Triangle::new(p1, p2, p3, None); 
        let t2 = Triangle::new(p3, p1, p2, None); 

        assert_eq!(t1.hash(), t2.hash());
    }

    #[test]
    fn test_genesis_triangle_is_canonical() {
        let g1 = Triangle::genesis();
        let expected_area = 1.299038105676658;
        assert!((g1.area() - expected_area).abs() < 1e-15, "Genesis triangle area is incorrect.");
    }

    #[test]
    fn test_subdivision_correctness() {
        let parent = setup_test_triangle();
        let parent_area = parent.area(); 
        let children = parent.subdivide();
        let total_child_area: Coord = children.iter().map(|t| t.area()).sum();
        
        assert!((total_child_area - parent_area * 0.75).abs() < 1e-9);
    }
    
    #[test]
    fn test_geometric_validation_valid() {
        let t = setup_test_triangle();
        assert!(t.is_valid(), "A normal triangle should be valid.");
        
        let g = Triangle::genesis();
        assert!(g.is_valid(), "The genesis triangle must be valid.");
    }

    #[test]
    fn test_geometric_validation_invalid_degenerate() {
        // Degenerate triangle: all points are collinear (on a straight line)
        let t_degenerate = Triangle::new(
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 3.0),
            None
        );
        assert!(!t_degenerate.is_valid(), "A degenerate (collinear) triangle should be invalid.");
    }
}
