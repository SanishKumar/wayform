pub type Point3 = [f64; 3];

#[derive(Clone, Debug)]
pub struct RegistrationControl {
    pub local: Point3,
    pub enu: Point3,
}

/// Geometry extracted after the host validates the local/ENU frame references.
#[derive(Clone, Debug)]
pub struct EntranceRegistration {
    pub local_y_down: bool,
    pub rotation_degrees: f64,
    pub translation_meters: Point3,
    pub uncertainty_meters: f64,
    pub evidence_id: String,
    pub controls: Vec<RegistrationControl>,
    pub local_endpoint: Point3,
    pub enu_endpoint: Point3,
}

fn distance(left: Point3, right: Point3) -> f64 {
    (left[0] - right[0])
        .hypot(left[1] - right[1])
        .hypot(left[2] - right[2])
}

impl EntranceRegistration {
    pub(crate) fn valid(&self) -> bool {
        let finite = |point: Point3| point.iter().all(|value| value.is_finite());
        self.rotation_degrees.is_finite()
            && self.uncertainty_meters.is_finite()
            && self.uncertainty_meters >= 0.0
            && finite(self.translation_meters)
            && finite(self.local_endpoint)
            && finite(self.enu_endpoint)
            && self.controls.len() >= 2
            && self
                .controls
                .iter()
                .all(|control| finite(control.local) && finite(control.enu))
            && self.controls.iter().skip(1).any(|control| {
                let first = &self.controls[0];
                (control.local[0] - first.local[0]).hypot(control.local[1] - first.local[1]) > 0.001
                    && (control.enu[0] - first.enu[0]).hypot(control.enu[1] - first.enu[1]) > 0.001
            })
    }

    fn project(&self, point: Point3) -> Point3 {
        let angle = self.rotation_degrees * std::f64::consts::PI / 180.0;
        let (x, y) = (
            point[0],
            point[1] * if self.local_y_down { -1.0 } else { 1.0 },
        );
        [
            angle.cos() * x - angle.sin() * y + self.translation_meters[0],
            angle.sin() * x + angle.cos() * y + self.translation_meters[1],
            point[2] + self.translation_meters[2],
        ]
    }

    pub(crate) fn error_meters(&self) -> f64 {
        let mut maximum = distance(self.project(self.local_endpoint), self.enu_endpoint);
        if !maximum.is_finite() {
            return f64::INFINITY;
        }
        for control in &self.controls {
            let residual = distance(self.project(control.local), control.enu);
            if !residual.is_finite() {
                return f64::INFINITY;
            }
            maximum = maximum.max(residual);
        }
        maximum + self.uncertainty_meters
    }
}
