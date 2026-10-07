use crate::{AppError, Entity, Geometry, PlateParameters};

pub fn validate(p: &PlateParameters) -> Result<(), AppError> {
    let values = [
        p.width,
        p.height,
        p.hole_diameter,
        p.edge_offset,
        p.origin[0],
        p.origin[1],
    ];
    if p.schema_version != 1
        || values
            .iter()
            .any(|n| !n.is_finite() || n.abs() > 1_000_000.)
    {
        return Err(AppError::new(
            "INVALID_PARAMETERS",
            "Sai phiên bản hoặc số vượt giới hạn.",
        ));
    }
    let r = p.hole_diameter / 2.;
    if r <= 0.
        || p.edge_offset <= r
        || p.width - 2. * p.edge_offset <= 2. * r
        || p.height - 2. * p.edge_offset <= 2. * r
    {
        return Err(AppError::new(
            "INVALID_GEOMETRY",
            "Lỗ phải nằm trong tấm và không giao nhau.",
        ));
    }
    Ok(())
}
pub fn plan(p: PlateParameters, object_id: &str) -> Result<Geometry, AppError> {
    validate(&p)?;
    let [x, y] = p.origin;
    let (w, h, e) = (p.width, p.height, p.edge_offset);
    let mut entities = vec![Entity::Polyline {
        id: format!("{object_id}:outline"),
        layer: "AI_OUTLINE".into(),
        points: [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
        closed: true,
    }];
    for (i, center) in [
        [x + e, y + e],
        [x + w - e, y + e],
        [x + w - e, y + h - e],
        [x + e, y + h - e],
    ]
    .into_iter()
    .enumerate()
    {
        entities.push(Entity::Circle {
            id: format!("{object_id}:hole_{i}"),
            layer: "AI_HOLES".into(),
            center,
            radius: p.hole_diameter / 2.,
        });
    }
    if p.add_dimensions {
        for (role, measurement) in [("width", w), ("height", h), ("diameter", p.hole_diameter)] {
            entities.push(Entity::Dimension {
                id: format!("{object_id}:dim_{role}"),
                layer: "AI_DIM".into(),
                measurement,
                role: role.into(),
            });
        }
    }
    Ok(Geometry {
        parameters: p,
        entities,
    })
}
/// Compare observed geometry to the deterministic plan with explicit tolerance.
pub fn verify(g: &Geometry, object_id: &str) -> Result<(), AppError> {
    validate(&g.parameters)?;
    let expected = plan(g.parameters.clone(), object_id)?;
    let actual = serde_json::to_value(g)
        .map_err(|_| AppError::new("VALIDATION", "Không đọc được hình học."))?;
    let expected = serde_json::to_value(expected)
        .map_err(|_| AppError::new("VALIDATION", "Không tạo được kiểm tra."))?;
    fn near(a: &serde_json::Value, b: &serde_json::Value) -> bool {
        match (a, b) {
            (serde_json::Value::Number(x), serde_json::Value::Number(y)) => {
                match (x.as_f64(), y.as_f64()) {
                    (Some(x), Some(y)) => (x - y).abs() <= 1e-7,
                    _ => false,
                }
            }
            (serde_json::Value::Array(x), serde_json::Value::Array(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(x, y)| near(x, y))
            }
            (serde_json::Value::Object(x), serde_json::Value::Object(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .all(|(k, v)| y.get(k).is_some_and(|other| near(v, other)))
            }
            _ => a == b,
        }
    }
    if !near(&actual, &expected) {
        return Err(AppError::new(
            "READBACK_MISMATCH",
            "Hình học đọc lại không khớp phương án.",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plate_and_resize() {
        let mut p = PlateParameters::default();
        let g = plan(p.clone(), "object").unwrap();
        assert_eq!(g.entities.len(), 8);
        verify(&g, "object").unwrap();
        p.width = 240.;
        let resized = plan(p, "object").unwrap();
        match &resized.entities[2] {
            Entity::Circle { center, radius, .. } => {
                assert_eq!(*center, [220., 20.]);
                assert_eq!(*radius, 5.);
            }
            _ => panic!("circle"),
        }
        let mut bad = resized;
        if let Entity::Circle { radius, .. } = &mut bad.entities[2] {
            *radius = 6.;
        }
        assert!(verify(&bad, "object").is_err());
    }
    #[test]
    fn rejects_invalid_parameters() {
        for value in [f64::NAN, f64::INFINITY, -1., 0., 50., 1_000_001.] {
            let p = PlateParameters {
                width: value,
                ..PlateParameters::default()
            };
            assert!(validate(&p).is_err());
        }
        let p = PlateParameters {
            edge_offset: 5.,
            ..PlateParameters::default()
        };
        assert!(validate(&p).is_err());
    }
}
