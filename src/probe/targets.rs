pub struct Targets {
    pub clipped_max: f32,
    pub crushed_max: f32,
    pub contrast_min: f32,
    pub raw_hard_edge_length_max: f32,
    pub box_like_min_extent: f32,
    pub faceted_pixels_per_triangle_max: f32,
    pub overdense_pixels_per_triangle_min: f32,
    pub penetration_max: f32,
}

impl Default for Targets {
    fn default() -> Targets {
        Targets {
            clipped_max: 0.03,
            crushed_max: 0.12,
            contrast_min: 0.22,
            raw_hard_edge_length_max: 0.1,
            box_like_min_extent: 0.3,
            faceted_pixels_per_triangle_max: 150.0,
            overdense_pixels_per_triangle_min: 0.6,
            penetration_max: 0.02,
        }
    }
}
